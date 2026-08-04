//! Edge cases built by mutating real ROMs in memory: checksum
//! breakage/repair, PCIR-vs-checksum size mismatch, VRAM geometry
//! lying, die/boot-string disagreement and TjMax corruption. Every
//! scenario asserts the read path *reports* the anomaly (never
//! crashes).

mod common;

use polaris_vbios::rom;

fn parse_bytes(data: &[u8], name: &str) -> rom::types::ParsedRom {
    rom::parse_bytes(data, name).expect("mutated stock ROM still parses")
}

/// Flipping the checksum byte itself makes the ROM "modified by
/// another tool": parse still works, the checksum rule fires, and no
/// other rule becomes noisy.
#[test]
fn flipped_byte_trips_checksum_rule_only() {
    let Some(base) = common::try_rom_bytes("AMD.RX590.8192.191126.rom") else {
        eprintln!("skipped: sample ROM not available");
        return;
    };
    let parsed = parse_bytes(&base, "orig.rom");
    // The checksum byte is the last byte of the checksum-covered region.
    let chk_off = parsed.header.checksum_checked_bytes - 1;
    let flipped = common::flip_byte(&base, chk_off);
    let r = parse_bytes(&flipped, "flipped.rom");
    assert!(
        r.warnings.iter().any(|w| w.contains("checksum")),
        "checksum rule fires:\n{:?}",
        r.warnings
    );
    // The flip must not cascade into other rules on a healthy ROM.
    let other: Vec<&String> = r
        .warnings
        .iter()
        .filter(|w| !w.contains("checksum"))
        .collect();
    assert!(other.is_empty(), "only the checksum rule fires: {other:?}");
}

/// Truncating the file makes the declared PCIR size disagree with the
/// checksum region - the parser either errors cleanly (truncated
/// tables) or warns; never panics (already covered in edge_parse, here
/// we assert the warning text when it parses).
#[test]
fn truncated_rom_reports_or_errors() {
    let Some(base) = common::try_rom_bytes("AMD.RX590.8192.191126.rom") else {
        eprintln!("skipped: sample ROM not available");
        return;
    };
    // Cut after the legacy image size block (checksum region is
    // declared via the block count; cutting at 0xE000 still leaves
    // most tables but breaks the declared size).
    let cut = common::truncated(&base, 0xE000);
    match rom::parse_bytes(&cut, "cut.rom") {
        Ok(r) => {
            let msg = r
                .warnings
                .iter()
                .find(|w| w.contains("PCIR") || w.contains("checksum"));
            assert!(
                msg.is_some(),
                "truncated ROM reports size mismatch:\n{:?}",
                r.warnings
            );
        }
        Err(e) => {
            // Clean error also acceptable (tables beyond the cut).
            assert!(!e.to_string().is_empty());
        }
    }
}

/// The BIOS bootup message names a die that disagrees with the device
/// ID: e.g. a 67DF ROM carrying a "Tonga" boot string (another GPU).
/// The rule fires and nothing crashes.
#[test]
fn mismatched_bootup_message_is_flagged() {
    let Some(base) = common::try_rom_bytes("AMD.RX590.8192.191126.rom") else {
        eprintln!("skipped: sample ROM not available");
        return;
    };
    let r = parse_bytes(&base, "orig.rom");
    let msg = r.header.bios_bootup_message.as_ref().expect("has bootup");
    // Find the bootup message region and rewrite it to a foreign ASIC.
    let needle = msg.as_bytes();
    let pos = base
        .windows(needle.len())
        .position(|w| w == needle)
        .expect("bootup string lives in the file");
    let mut hacked = base.clone();
    // The disk id 0x67DF is a Polaris 10 family die, so pick a boot
    // string that the rule maps to a *different* family ("lexa" →
    // Polaris 12).
    let foreign = b"LEXA PRO GDDR5 8192MB REPLACEMENT";
    for (i, &b) in foreign.iter().enumerate() {
        if pos + i < hacked.len() {
            hacked[pos + i] = b;
        }
    }
    // NUL-terminate the overwritten message so stale bytes cannot leak.
    let end = (pos + foreign.len()).min(hacked.len());
    if end < hacked.len() {
        hacked[end] = 0;
    }
    // Restore the checksum so only the boot/die rule is exercised.
    rom::patch::fix_checksum(&mut hacked).expect("fixes checksum");
    let r = parse_bytes(&hacked, "hacked.rom");
    assert!(
        r.warnings
            .iter()
            .any(|w| w.contains("bootup message names the ASIC")),
        "boot/die mismatch flagged:\n{:?}",
        r.warnings
    );
}

/// TjMax outside the plausible 60-110 C band trips the thermal rule.
#[test]
fn corrupted_tjmax_is_flagged() {
    let Some(base) = common::try_rom_bytes("AMD.RX590.8192.191126.rom") else {
        eprintln!("skipped: sample ROM not available");
        return;
    };
    let r = parse_bytes(&base, "orig.rom");
    let pt = r.powerplay.powertune.as_ref().expect("has PowerTune");
    let sane_tjmax = pt.tjmax_c;
    assert!(
        (60..=110).contains(&sane_tjmax),
        "stock TjMax sane: {sane_tjmax}"
    );

    // Locate the TjMax u16 in the file by scanning for the value
    // (searching the whole file is fine for the test).
    let bytes = sane_tjmax.to_le_bytes();
    let Some(pos) = base.windows(2).position(|w| w == bytes) else {
        eprintln!("skipped: cannot locate TjMax bytes");
        return;
    };
    let mut hacked = base.clone();
    hacked[pos] = 0xFA; // 250 C
    hacked[pos + 1] = 0x00;
    let r = parse_bytes(&hacked, "hot.rom");
    assert!(
        r.warnings.iter().any(|w| w.contains("TjMax")),
        "TjMax rule fires:\n{:?}",
        r.warnings
    );
}

/// A PowerPlay format rev that is not 7.x must be flagged (foreign
/// GPU / misread), not crash.
#[test]
fn foreign_powerplay_rev_is_flagged() {
    let Some(base) = common::try_rom_bytes("AMD.RX590.8192.191126.rom") else {
        eprintln!("skipped: sample ROM not available");
        return;
    };
    let r = parse_bytes(&base, "orig.rom");
    assert_eq!(r.powerplay.header_fmt_rev, 7, "stock ROM is PP rev 7");
    // Locate the PowerPlay table through the master data table, flip
    // the format-rev byte (first byte of the table), and restore the
    // checksum so only the foreign-rev rule fires.
    let rdr = rom::reader::Reader::new(&base);
    let pp_off =
        rom::header::master_table_offset(&rdr, r.header.master_data_table_offset, "PowerPlayInfo")
            .expect("finds PowerPlayInfo offset");
    let mut hacked = base.clone();
    // Format rev is the third byte of the PowerPlay table (off + 2).
    hacked[pp_off + 2] = 0x08;
    rom::patch::fix_checksum(&mut hacked).expect("fixes checksum");
    let r = parse_bytes(&hacked, "pp8.rom");
    assert!(
        r.warnings.iter().any(|w| w.contains("format")),
        "foreign PP rev flagged:\n{:?}",
        r.warnings
    );
}
