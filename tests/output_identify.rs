//! Output tests for `identify`: text one-liner and `--json`, including
//! the die-detection labels (regression for the Polaris 30/20/10
//! split) and the warnings that must surface with exit code 2.

mod common;

use common::exit_is;

const RX590: &str = "AMD.RX590.8192.191126.rom";
const RX570: &str = "AMD.RX570.4096.170424.rom";
const RX550_VDDC: &str = "Sapphire.RX550.2048.170504.rom";

/// A ROM with validation warnings must exit 2 and surface the warning
/// text in JSON - the scriptable contract.
#[test]
fn identify_json_exits_2_with_warnings() {
    if common::try_rom(RX550_VDDC).is_none() {
        eprintln!("skipped: {RX550_VDDC} not available");
        return;
    }
    let out = common::run(&[
        "identify",
        "--json",
        common::rom(RX550_VDDC).to_str().unwrap(),
    ]);
    assert_eq!(out.1, 2, "warnings must exit 2 (stderr: {})", out.2);
    assert!(
        common::stdout(&out).contains("highest VDDC LUT entry is 1100 mV"),
        "VDDC warning in JSON"
    );
}

/// Text mode: one line with the die label, TDP and a clean/tripped
/// marker. The default color palette is for interactive use; the
/// `--no-color` flag must yield plain text with no ANSI codes.
#[test]
fn identify_text_one_liner() {
    if common::try_rom(RX590).is_none() {
        eprintln!("skipped: {RX590} not available");
        return;
    }
    let out = common::run(&[
        "identify",
        "--no-color",
        common::rom(RX590).to_str().unwrap(),
    ]);
    assert!(exit_is(&out, 0));
    let s = common::stdout(&out);
    assert!(s.contains("RX 590 (Polaris 30)"), "die label in text:\n{s}");
    assert!(s.contains("TDP 185W"), "TDP in text:\n{s}");
    assert!(!s.contains('\u{1b}'), "no ANSI escapes with --no-color");
}

/// `identify` accepts several ROMs at once and reports each.
#[test]
fn identify_multi_rom() {
    if common::try_rom(RX590).is_none() || common::try_rom(RX570).is_none() {
        eprintln!("skipped: sample ROMs not available");
        return;
    }
    let a = common::rom(RX590);
    let b = common::rom(RX570);
    let out = common::run(&[
        "identify",
        "--json",
        a.to_str().unwrap(),
        b.to_str().unwrap(),
    ]);
    assert!(exit_is(&out, 0));
    let s = common::stdout(&out);
    assert_eq!(s.matches("\"file\":").count(), 2, "two entries:\n{s}");
    assert!(s.contains("RX 590 (Polaris 30)") && s.contains("RX 570/580 (Polaris 20)"));
}
