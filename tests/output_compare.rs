//! `compare` contract, driven by the stable JSON output: 0 = identical,
//! 1 = differs, 2 = identical but warnings. Text rendering is only
//! smoke-checked (the ANSI table is presentation, not contract).

mod common;

use common::exit_is;
use std::path::PathBuf;

fn roms() -> Option<(PathBuf, PathBuf)> {
    let a = common::try_rom("AMD.RX480.8192.160603.rom")?;
    let b = common::try_rom("AMD.RX480.8192.160603_1.rom")?;
    Some((a, b))
}

fn parse_json(s: &str) -> serde_json::Value {
    serde_json::from_str(s).unwrap_or_else(|e| panic!("invalid JSON from compare: {e}\n{s}"))
}

/// The JSON side documents carry `file_name` (derived from the CLI
/// path, not the ROM content) - strip it before comparing sides.
fn side(v: &serde_json::Value, name: &str) -> serde_json::Value {
    let mut s = v[name].clone();
    if let serde_json::Value::Object(map) = &mut s {
        map.remove("file_name");
    }
    s
}

#[test]
fn compare_identical_exits_0() {
    let Some((a, _)) = roms() else {
        eprintln!("skipped: sample ROMs not available");
        return;
    };
    // Copy the same file under a second name: identical bytes.
    let b = common::temp_path("copy-of-a.rom");
    std::fs::copy(&a, &b).unwrap();
    let out = common::run(&[
        "compare",
        "--json",
        a.to_str().unwrap(),
        b.to_str().unwrap(),
    ]);
    assert!(
        exit_is(&out, 0),
        "byte-identical ROMs exit 0 (out: {})",
        out.0
    );
    let v = parse_json(common::stdout(&out));
    assert_eq!(
        side(&v, "a"),
        side(&v, "b"),
        "identical ROMs serialize to the same document"
    );
}

/// The two AMD RX 480 samples are different BIOSes (262 KiB vs 512
/// KiB); compare must exit 1 and the JSON sides must differ.
#[test]
fn compare_different_exits_1() {
    let Some((a, b)) = roms() else {
        eprintln!("skipped: sample ROMs not available");
        return;
    };
    let out = common::run(&[
        "compare",
        "--json",
        a.to_str().unwrap(),
        b.to_str().unwrap(),
    ]);
    assert!(exit_is(&out, 1), "different ROMs exit 1 (out: {})", out.0);
    let v = parse_json(common::stdout(&out));
    assert_ne!(
        side(&v, "a"),
        side(&v, "b"),
        "different ROMs serialize differently"
    );
    assert_eq!(
        v["a"]["header"]["file_size"], 524288,
        "side a is the 512 KiB sample"
    );
    assert_eq!(
        v["b"]["header"]["file_size"], 262144,
        "side b is the 256 KiB sample"
    );
}

/// Text mode still renders a report with the diff marker (smoke test -
/// the authoritative verdict is the exit code + JSON above).
#[test]
fn compare_text_smoke() {
    let Some((a, b)) = roms() else {
        eprintln!("skipped: sample ROMs not available");
        return;
    };
    let out = common::run(&["compare", a.to_str().unwrap(), b.to_str().unwrap()]);
    assert!(exit_is(&out, 1));
    let s = common::stdout(&out);
    assert!(
        s.contains('≠'),
        "text mode shows the ≠ difference marker:\n{s}"
    );
    assert!(
        s.contains("File size (bytes)"),
        "the size diff is reported:\n{s}"
    );
}

/// When the two ROMs are byte-identical but carry validation warnings,
/// compare exits 2 (identical, warnings) - the documented contract.
#[test]
fn compare_identical_with_warnings_exits_2() {
    let src = common::try_rom("Sapphire.RX550.2048.170504.rom");
    let Some(src) = src else {
        eprintln!("skipped: sample ROM not available");
        return;
    };
    let a = common::temp_path("warn-a.rom");
    let b = common::temp_path("warn-b.rom");
    std::fs::copy(&src, &a).unwrap();
    std::fs::copy(&src, &b).unwrap();
    let out = common::run(&["compare", a.to_str().unwrap(), b.to_str().unwrap()]);
    assert_eq!(out.1, 2, "identical-with-warnings exits 2 (out: {})", out.0);
}

#[test]
fn compare_missing_file_errors() {
    let Some(a) = common::try_rom("AMD.RX590.8192.191126.rom") else {
        eprintln!("skipped: sample ROM not available");
        return;
    };
    let ghost = common::temp_path("nope.rom");
    let out = common::run(&["compare", a.to_str().unwrap(), ghost.to_str().unwrap()]);
    assert_ne!(out.1, 0, "missing side must not look identical");
    assert!(out.2.contains("error"), "stderr has the error: {}", out.2);
}
