//! Output tests for `dump`: JSON fields, CSV only for the tabular
//! sections, rejection of unknown sections, and `-o` writing.

mod common;

use common::exit_is;
use std::path::PathBuf;

fn requires_rom() -> Option<PathBuf> {
    common::try_rom("AMD.RX590.8192.191126.rom")
}

#[test]
fn dump_json_reference_rom() {
    let Some(rom) = requires_rom() else {
        eprintln!("skipped: sample ROM not available");
        return;
    };
    let out = common::run(&[
        "dump",
        "--json",
        "--sections",
        "header,powertune",
        rom.to_str().unwrap(),
    ]);
    assert!(exit_is(&out, 0), "dump --json exits 0: {:?}", out.2);
    let s = common::stdout(&out);
    assert!(
        s.contains("\"file_name\": \"AMD.RX590.8192.191126.rom\""),
        "file_name:\n{s}"
    );
    assert!(s.contains("\"checksum_valid\": true"), "checksum:\n{s}");
    assert!(s.contains("\"tdp_w\": 185"), "PowerTune TDP:\n{s}");
}

/// The JSON carries the header identity: ATOM format rev, main table
/// offset, bootup message and vendor - the canary of a healthy parse.
#[test]
fn dump_json_header_fields_present() {
    let Some(rom) = requires_rom() else {
        eprintln!("skipped: sample ROM not available");
        return;
    };
    let out = common::run(&["dump", "--json", rom.to_str().unwrap()]);
    assert!(exit_is(&out, 0));
    let s = common::stdout(&out);
    for needle in [
        "\"master_data_table_offset\":",
        "\"atom_fmt_rev\":",
        "\"subsystem_vendor_name\": \"AMD/ATI",
        "\"bios_bootup_message\": \"50mv C94441 POLARIS 30 XT A1",
    ] {
        assert!(s.contains(needle), "missing {needle:?} in dump output");
    }
}

/// CSV works for the tabular sections (straps) and gives a header + at
/// least one data row.
#[test]
fn csv_straps_is_tabular() {
    let Some(rom) = requires_rom() else {
        eprintln!("skipped: sample ROM not available");
        return;
    };
    let out = common::run(&[
        "dump",
        "--format",
        "csv",
        "--sections",
        "straps",
        rom.to_str().unwrap(),
    ]);
    assert!(exit_is(&out, 0), "csv exits 0 (err: {})", out.2);
    let s = common::stdout(&out);
    let lines: Vec<&str> = s.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(lines.len() >= 2, "csv has header + data:\n{s}");
    assert!(lines[0].contains(','), "csv header has commas:\n{s}");
}

#[test]
fn csv_non_tabular_section_rejected() {
    let Some(rom) = requires_rom() else {
        eprintln!("skipped: sample ROM not available");
        return;
    };
    let out = common::run(&[
        "dump",
        "--format",
        "csv",
        "--sections",
        "header",
        rom.to_str().unwrap(),
    ]);
    assert_eq!(out.1, 1, "csv of 'header' must fail (out: {})", out.0);
    assert!(
        out.2.contains("not exportable as CSV"),
        "message on stderr:\n{}",
        out.2
    );
}

#[test]
fn unknown_section_rejected() {
    let Some(rom) = requires_rom() else {
        eprintln!("skipped: sample ROM not available");
        return;
    };
    let out = common::run(&["dump", "--sections", "nope", rom.to_str().unwrap()]);
    assert_eq!(out.1, 1, "unknown section should exit 1 (out: {})", out.0);
    assert!(out.2.contains("unknown section"), "message:\n{}", out.2);
}

/// `-o <file>` redirects the output to disk.
#[test]
fn dump_json_to_file() {
    let Some(rom) = requires_rom() else {
        eprintln!("skipped: sample ROM not available");
        return;
    };
    let dest = common::temp_path("dump-out.json");
    let out = common::run(&[
        "dump",
        "--json",
        "-o",
        dest.to_str().unwrap(),
        rom.to_str().unwrap(),
    ]);
    assert!(exit_is(&out, 0), "dump -o exits 0 (err: {})", out.2);
    assert!(out.0.is_empty(), "with -o output goes to file, not stdout");
    let written = std::fs::read_to_string(&dest).unwrap();
    assert!(written.contains("\"file_name\""), "file was written");
}
