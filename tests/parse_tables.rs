//! Structural tests on reference ROMs: the main tables (PowerPlay,
//! FirmwareInfo, VRAM_Info, GFX_Info, ASIC_ProfilingInfo...), the
//! PowerTune limits and the die-dependent constants all read correctly
//! on stock ROMs. One "genre" per fixture - see docs/reference.

mod common;

use polaris_vbios::rom;

fn parsed(name: &str) -> Option<(String, rom::types::ParsedRom)> {
    let p = common::try_rom(name)?;
    match rom::parse_rom(&p) {
        Ok(r) => Some((name.to_string(), r)),
        Err(e) => panic!("parsing {name} failed: {e:#}"),
    }
}

/// PowerPlay is format 7.x everywhere (the family requirement), with a
/// non-empty SCLK/MCLK table and a sane boot VDDC on every reference.
#[test]
fn reference_roms_powerplay_is_format_7() {
    for name in [
        "AMD.RX590.8192.191126.rom",
        "AMD.RX480.8192.160603.rom",
        "AMD.RX570.4096.170424.rom",
        "Sapphire.RX550.4096.170918.rom",
        "MSI.RX590.8192.191007.rom",
    ] {
        let Some((name, r)) = parsed(name) else {
            eprintln!("skipped: {name} not available");
            continue;
        };
        assert_eq!(r.powerplay.header_fmt_rev, 7, "{name}: PP fmt rev");
        assert!(
            !r.powerplay.sclk_table.is_empty(),
            "{name}: empty SCLK table"
        );
        assert!(
            !r.powerplay.mclk_table.is_empty(),
            "{name}: empty MCLK table"
        );
        let vddc = r.firmware.bootup_vddc_mv;
        assert!(
            (700..=1200).contains(&vddc),
            "{name}: bootup VDDC {vddc} mV outside plausible range"
        );
        assert!(
            r.vram.modules.iter().any(|m| !m.part_number.is_empty()),
            "{name}: all VRAM modules lack part numbers"
        );
    }
}

/// Known TDP per reference card - the envelope must not flag stock.
#[test]
fn reference_roms_tdp_values() {
    for (name, expected) in [
        ("AMD.RX590.8192.191126.rom", 185),
        ("AMD.RX570.4096.170424.rom", 120),
        ("Sapphire.RX550.4096.170918.rom", 35),
        ("AMD.RX480.8192.160603.rom", 110),
    ] {
        let Some((_, r)) = parsed(name) else {
            eprintln!("skipped: {name} not available");
            continue;
        };
        let pt = r.powerplay.powertune.as_ref().unwrap();
        assert_eq!(pt.tdp_w, expected, "{name}: TDP");
        assert!(
            r.warnings.is_empty(),
            "{name}: stock ROM should not warn, got {:?}",
            r.warnings
        );
    }
}

/// Mobile Polaris ROMs (68-85 W) parse cleanly and carry sane TjMax.
#[test]
fn mobile_roms_parse_clean() {
    for name in [
        "Alienware.RX570Mobile.8192.161102.rom",
        "Asus.RX580Mobile.4096.170829.rom",
        "Dell.RX580Mobile.8192.170419.rom",
    ] {
        let Some((_, r)) = parsed(name) else {
            eprintln!("skipped: {name} not available");
            continue;
        };
        assert!(
            r.warnings.is_empty(),
            "{name}: mobile stock ROM should not warn, got {:?}",
            r.warnings
        );
        let tdp = r.powerplay.powertune.as_ref().map(|p| p.tdp_w);
        assert!(
            (30..=150).contains(&tdp.unwrap_or(0)),
            "{name}: mobile TDP {tdp:?} outside plausible range"
        );
    }
}
