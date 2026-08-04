//! `check` exit-code contract: 0 = clean, 1 = findings, 2 = parse error.

mod common;

use common::exit_is;

#[test]
fn check_exit_0_clean_rom() {
    let Some(rom) = common::try_rom("AMD.RX590.8192.191126.rom") else {
        eprintln!("skipped: sample ROM not available");
        return;
    };
    let out = common::run(&["check", rom.to_str().unwrap()]);
    assert!(
        exit_is(&out, 0),
        "clean ROM must exit 0 (stderr: {})",
        out.2
    );
    assert!(
        out.0.trim().is_empty(),
        "clean ROM prints nothing on stdout"
    );
}

#[test]
fn check_exit_1_with_warnings() {
    let Some(rom) = common::try_rom("Sapphire.RX550.2048.170504.rom") else {
        eprintln!("skipped: sample ROM not available");
        return;
    };
    let out = common::run(&["check", rom.to_str().unwrap()]);
    assert!(
        exit_is(&out, 1),
        "VDDC-warning ROM must exit 1 (stdout: {})",
        out.0
    );
    assert!(out.0.contains("VDDC LUT entry"), "warning text on stdout");
}

/// check accepts several ROMs; any parse error wins and makes the exit
/// 2 (not 1), even when the other ROM is clean.
#[test]
fn check_exit_2_on_missing_file() {
    let ghost = common::temp_path("definitely-not-a-rom.rom");
    let out = common::run(&["check", ghost.to_str().unwrap()]);
    assert!(
        exit_is(&out, 2),
        "missing file must exit 2 (got {:?})",
        out.0
    );
}

#[test]
fn check_exit_2_on_garbage_file() {
    let rom = common::temp_path("garbage.rom");
    std::fs::write(&rom, vec![0xFFu8; 4096]).unwrap();
    let out = common::run(&["check", rom.to_str().unwrap()]);
    assert!(
        exit_is(&out, 2),
        "garbage file must exit 2 (got: {})",
        out.0
    );
}

/// Multi-ROM check: clean + warning mixes report findings but not
/// errors.
#[test]
fn check_multiple_roms_findings() {
    let Some(a) = common::try_rom("AMD.RX590.8192.191126.rom") else {
        eprintln!("skipped: sample ROM not available");
        return;
    };
    let Some(b) = common::try_rom("Yeston.RX550.4096.180112.rom") else {
        eprintln!("skipped: sample ROM not available");
        return;
    };
    let out = common::run(&["check", a.to_str().unwrap(), b.to_str().unwrap()]);
    assert!(
        exit_is(&out, 1),
        "mixed clean+warning -> 1 (got: {})",
        out.0
    );
}
