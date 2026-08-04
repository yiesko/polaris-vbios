//! Golden-output regression tests: the `identify --json` text of a set
//! of representative ROMs is committed under `tests/golden/` and
//! compared byte-for-byte on every run. When the parser or labels
//! change on purpose, regenerate with:
//!
//!     PBIOS_BLESS=1 cargo test --test regression
//!
//! (the tests then just overwrite the golden files). Accidental
//! changes show up here instead of silently changing the CLI output.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

/// The ROMs that lock the golden output. Chosen to cover the die zones
/// and the warning path: a Polaris 30 with a named boot, a Polaris 20,
/// a bare-boot 67DF (MSI 113-...), a Lexa, and the VDDC warning ROM.
const GOLDEN_ROMS: &[&str] = &[
    "AMD.RX590.8192.191126.rom",
    "AMD.RX570.4096.170424.rom",
    "MSI.RX590.8192.191007.rom",
    "Sapphire.RX550.4096.170918.rom",
    "Yeston.RX550.4096.180112.rom",
];

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
}

fn golden_path(name: &str) -> PathBuf {
    golden_dir().join(format!("{name}.identify.json"))
}

fn bless_requested() -> bool {
    std::env::var_os("PBIOS_BLESS").is_some()
}

#[test]
fn identify_json_goldens_match() {
    let _bless = bless_requested();
    fs::create_dir_all(golden_dir()).unwrap();
    let mut failures = Vec::new();
    for name in GOLDEN_ROMS {
        let Some(rom) = common::try_rom(name) else {
            eprintln!("skipped: {name} not available");
            continue;
        };
        let out = common::run(&["identify", "--json", rom.to_str().unwrap()]);
        // Exit code is part of the contract - lock it too.
        let text = format!("exit={}\n{}", out.1, out.0);
        let file = golden_path(name);
        if bless_requested() {
            fs::write(&file, &text).unwrap();
            continue;
        }
        match fs::read_to_string(&file) {
            Ok(expected) if expected == text => {}
            Ok(expected) => failures.push(format!(
                "{name}: golden mismatch\n--- repo ---\n{expected}--- actual ({}) ---\n{text}",
                out.1
            )),
            Err(_) if !file.exists() => failures.push(format!(
                "{name}: no golden file yet - run with PBIOS_BLESS=1 to create it"
            )),
            Err(e) => failures.push(format!("{name}: reading golden: {e}")),
        }
    }

    if bless_requested() {
        eprintln!(
            "PBIOS_BLESS=1: golden files written to {}",
            golden_dir().display()
        );
        return;
    }
    assert!(
        failures.is_empty(),
        "{} golden mismatch(es):\n{}\n(if the change is intentional, run tests with PBIOS_BLESS=1)",
        failures.len(),
        failures.join("\n")
    );
}
