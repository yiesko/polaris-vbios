//! Die-detection tests (limits::detect_die): named boot strings,
//! whitespace-insensitive "POLARIS 30", the MC-microcode fallback, the
//! anonymous 67DF boots -> EllesmereGeneric, and the 67DF invariant.

mod common;

use polaris_vbios::rom;
use polaris_vbios::rom::limits::{Die, detect_die};

fn die_of(name: &str) -> Die {
    let p = common::rom(name);
    let parsed = rom::parse_rom(&p).expect("sample ROM parses");
    detect_die(&parsed)
}

/// "C94441 POLARIS 30 XT A1 ..." must match Polaris 30 despite the
/// space in "POLARIS 30" (normalization strips whitespace).
#[test]
fn bootup_string_polaris30_with_space() {
    let _p = try_rom!("AMD.RX590.8192.191126.rom");
    assert_eq!(die_of("AMD.RX590.8192.191126.rom"), Die::Polaris30);
}

/// Boot strings naming Polaris20/Polaris10/... map to the right dies.
#[test]
fn bootup_string_named_dies() {
    for (name, expected) in [
        ("AMD.RX570.4096.170424.rom", Die::Ellesmere20),
        ("Asus.RX460.2048.160817.rom", Die::Baffin),
        ("Sapphire.RX550.4096.170918.rom", Die::Lexa),
    ] {
        if common::try_rom(name).is_none() {
            eprintln!("skipped: {name} not available");
            continue;
        }
        assert_eq!(die_of(name), expected, "{name} die");
    }
}

/// Anonymous 67DF boots (Asus "67DFHB...", MSI "113-MSI...") have no
/// die name - the MC microcode (12 nm Polaris 30) or the P10/P20 union
/// must kick in, never Unknown.
#[test]
fn anonymous_67df_falls_to_generic_or_p30() {
    for name in [
        "MSI.RX590.8192.191007.rom",      // 113-MSI..., 220 W -> Polaris 30
        "Asus.RX470.4096.160715.rom",     // 67DFHB..., 85 W -> EllesmereGeneric
        "Gigabyte.RX580.8192.170329.rom", // GV-RX580XTRAORUS... -> P20 union
        "Sapphire.RX570.8192.180123.rom", // E347/E353... -> Generic (or P20 via ucode)
    ] {
        let Some(p) = common::try_rom(name) else {
            eprintln!("skipped: {name} not available");
            continue;
        };
        let parsed = rom::parse_rom(&p).expect("parses");
        let die = detect_die(&parsed);
        assert_ne!(
            die,
            Die::Unknown,
            "{name}: anonymous 67DF must not be Unknown (got {die:?})"
        );
    }
}

/// Every 67DF ROM in the collection maps to a known die (never
/// Unknown) - covers the whole corpus, including the mobile ones.
#[test]
fn every_67df_is_known() {
    let unknown: Vec<String> = common::all_roms()
        .into_iter()
        .filter_map(|p| {
            let parsed = rom::parse_rom(&p).ok()?;
            let device = parsed.pci_images.first()?.device_id;
            if device == 0x67DF && detect_die(&parsed) == Die::Unknown {
                Some(p.file_name().unwrap().to_string_lossy().into_owned())
            } else {
                None
            }
        })
        .collect();
    assert!(
        unknown.is_empty(),
        "67DF ROMs with unknown die: {unknown:?}"
    );
}

/// MC-microcode gate: a 67DF without a die-naming boot string but with
/// a 12 nm ucode (>= 11853696) is Polaris 30 - the RX 590 MSI case.
#[test]
fn mcu_code_separates_polaris30() {
    let p = try_rom!("MSI.RX590.8192.191007.rom");
    let parsed = rom::parse_rom(&p).expect("parses");
    let ucode = parsed
        .vram
        .mcu_code_version
        .expect("MSI RX 590 has MC ucode");
    assert!(
        ucode >= 11_853_696,
        "MSI RX 590 ucode {ucode} is not in the 12 nm range"
    );
    assert_eq!(detect_die(&parsed), Die::Polaris30);
}
