//! Smoke tests over the whole sample collection: every ROM must parse
//! without crashing, report the expected validation baseline (exactly
//! the four known VDDC-LUT warnings) and carry a valid checksum.

mod common;

use polaris_vbios::rom;

/// Stock VDDC-LUT warnings seen on factory ROMs, deliberately kept as
/// warnings (see docs/reference/README.md, Validation section). The
/// exact file/message tuple is the regression baseline: if the parser
/// or the rules change, this test says which ROMs are affected.
const KNOWN_VDDC_WARNINGS: &[(&str, &str)] = &[
    (
        "Sapphire.RX550.2048.170504.rom",
        "highest VDDC LUT entry is 1100 mV, above the die maximum of 1075 mV",
    ),
    (
        "Sapphire.RX570.8192.180123_1.rom",
        "highest VDDC LUT entry is 1175 mV, above the die maximum of 1150 mV",
    ),
    (
        "Sapphire.RX570.8192.180123.rom",
        "highest VDDC LUT entry is 1175 mV, above the die maximum of 1150 mV",
    ),
    (
        "Yeston.RX550.4096.180112.rom",
        "highest VDDC LUT entry is 1100 mV, above the die maximum of 1075 mV",
    ),
];

/// Every sample ROM parses successfully (no crash, no parse error) and
/// the validation baseline holds: the only warnings are the four known
/// VDDC-LUT ones, on the exact files, with the exact text.
#[test]
fn all_sample_roms_parse_and_match_warning_baseline() {
    let roms = common::all_roms();
    if roms.is_empty() {
        eprintln!("skipped: sample collection unavailable");
        return;
    }

    let mut failures = Vec::new();
    for p in &roms {
        match rom::parse_rom(p) {
            Ok(parsed) => {
                let name = p.file_name().unwrap().to_string_lossy().to_string();
                let expected = KNOWN_VDDC_WARNINGS
                    .iter()
                    .find(|(n, _)| *n == name)
                    .map(|(_, msg)| *msg);
                match (parsed.warnings.is_empty(), expected) {
                    (false, None) => failures.push(format!(
                        "{name}: unexpected warnings: {:?}",
                        parsed.warnings
                    )),
                    (false, Some(msg)) => {
                        if parsed.warnings.len() != 1 || !parsed.warnings[0].starts_with(msg) {
                            failures.push(format!(
                                "{name}: expected VDDC warning starting with {msg:?}, got {:?}",
                                parsed.warnings
                            ));
                        }
                    }
                    (true, Some(msg)) => failures.push(format!(
                        "{name}: expected the known VDDC warning ({msg:?}) but it disappeared"
                    )),
                    (true, None) => {}
                }
            }
            Err(e) => failures.push(format!("{}: parse error: {e:#}", p.display())),
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {} ROMs deviated from the baseline:\n{}",
        failures.len(),
        roms.len(),
        failures.join("\n")
    );
}
