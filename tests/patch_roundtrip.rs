//! Round-trip tests for `patch`: apply an edit, re-parse/check the
//! result (checksum recomputed, no warnings), and the guardrails
//! (absurd values refused without --force, hard-limit clocks never
//! forced, --dry-run writes nothing, --out must differ from source).

mod common;

use common::exit_is;
use std::path::{Path, PathBuf};

const RX570: &str = "AMD.RX570.4096.170424.rom";

fn work_rom(name: &str) -> PathBuf {
    let src = common::rom_bytes(name);
    // Use a unique suffix to avoid collisions when tests run in parallel.
    let unique = format!(
        "{}-{}",
        name.replace('.', "_"),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let p = common::temp_path(&unique);
    std::fs::write(&p, &src).unwrap();
    p
}

fn rom_is_valid(path: &Path) -> bool {
    let out = common::run(&["check", path.to_str().unwrap()]);
    out.1 == 0
}

/// Patch --pp-tdp 135 -> exit 0, output written, checksum valid,
/// TDP read back as 135.
#[test]
fn patch_tdp_roundtrip() {
    if common::try_rom(RX570).is_none() {
        eprintln!("skipped: sample ROM not available");
        return;
    }
    let src = work_rom(RX570);
    let out = common::temp_path("patched.rom");
    let r = common::run(&[
        "patch",
        src.to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
        "--pp-tdp",
        "135",
    ]);
    assert!(exit_is(&r, 0), "patch exits 0: {:?}", r);
    assert!(out.is_file(), "output written");
    assert!(rom_is_valid(&out), "patched ROM checks out clean");
    assert!(
        rom_is_valid(&src),
        "source ROM is untouched and still clean"
    );
    // read the TDP back via identify
    let id = common::run(&["identify", "--json", out.to_str().unwrap()]);
    assert!(id.0.contains("\"tdp_w\": 135"), "TDP read back: {}", id.0);
    // patched bytes differ from source (edit + checksum byte)
    let a = std::fs::read(&src).unwrap();
    let b = std::fs::read(&out).unwrap();
    assert_ne!(a, b, "patched file differs from source");
}

/// --dry-run prints the plan but writes nothing.
#[test]
fn patch_dry_run_writes_nothing() {
    if common::try_rom(RX570).is_none() {
        eprintln!("skipped: sample ROM not available");
        return;
    }
    let src = work_rom(RX570);
    let out = common::temp_path("dry.rom");
    let r = common::run(&[
        "patch",
        src.to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
        "--pp-tdp",
        "135",
        "--dry-run",
    ]);
    assert!(exit_is(&r, 0), "dry run exits 0");
    assert!(
        r.0.contains("dry run - nothing written"),
        "dry run marker: {}",
        r.0
    );
    assert!(!out.exists(), "dry run must not create the output");
}

/// --out equal to the source is refused (never patch in place).
#[test]
fn patch_out_same_as_source_refused() {
    if common::try_rom(RX570).is_none() {
        eprintln!("skipped: sample ROM not available");
        return;
    }
    let src = work_rom(RX570);
    let before = std::fs::read(&src).unwrap();
    let r = common::run(&[
        "patch",
        src.to_str().unwrap(),
        "--out",
        src.to_str().unwrap(),
        "--pp-tdp",
        "135",
    ]);
    assert_ne!(r.1, 0, "in-place patch must fail");
    assert!(
        r.2.contains("never patch in place"),
        "error message: {}",
        r.2
    );
    let after = std::fs::read(&src).unwrap();
    assert_eq!(before, after, "source untouched");
}

/// Absurd TDP (5 W, 800 W) is refused without --force and the output
/// is not created; --force writes it (the guardrail yields, the
/// checksum still lands).
#[test]
fn patch_absurd_tdp_requires_force() {
    if common::try_rom(RX570).is_none() {
        eprintln!("skipped: sample ROM not available");
        return;
    }
    for watts in ["5", "800"] {
        let src = work_rom(RX570);
        let out = common::temp_path(&format!("out-{watts}.rom"));
        let r = common::run(&[
            "patch",
            src.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
            "--pp-tdp",
            watts,
        ]);
        assert_eq!(r.1, 1, "TDP {watts} W refused without --force");
        assert!(!out.exists(), "refused patch writes nothing");
        assert!(
            r.2.contains("--force") || r.2.contains("error"),
            "stderr explains: {}",
            r.2
        );

        // With --force it goes through.
        let src2 = work_rom(RX570);
        let out2 = common::temp_path(&format!("out-{watts}-f.rom"));
        let r2 = common::run(&[
            "patch",
            src2.to_str().unwrap(),
            "--out",
            out2.to_str().unwrap(),
            "--pp-tdp",
            watts,
            "--force",
        ]);
        assert!(exit_is(&r2, 0), "--force writes TDP {watts}: {:?}", r2);
        assert!(out2.is_file());
    }
}

/// A clock outside the representable range is refused even with
/// --force (clock hard limits are never bypassed).
#[test]
fn patch_implausible_clock_refused_even_with_force() {
    if common::try_rom(RX570).is_none() {
        eprintln!("skipped: sample ROM not available");
        return;
    }
    for force in [false, true] {
        let src = work_rom(RX570);
        let out = common::temp_path(if force { "clk-f.rom" } else { "clk.rom" });
        let mut args = vec![
            "patch",
            src.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
            "--pp-sclk",
            "0",
            "99999",
        ];
        if force {
            args.push("--force");
        }
        let r = common::run(&args);
        assert_eq!(r.1, 1, "99999 MHz refused (force={force})");
        assert!(!out.exists(), "no output written");
    }
}

/// --fix-checksum repairs a broken ROM: the output parses clean and
/// carries a valid checksum.
#[test]
fn patch_fix_checksum_repairs() {
    if common::try_rom(RX570).is_none() {
        eprintln!("skipped: sample ROM not available");
        return;
    }
    let bytes = common::rom_bytes(RX570);
    let broken = common::flip_byte(&bytes, 0x9E84);
    let src = common::temp_path("broken.rom");
    std::fs::write(&src, &broken).unwrap();

    let before = common::run(&["check", src.to_str().unwrap()]);
    assert_eq!(before.1, 1, "broken ROM trips the checksum rule");

    let out = common::temp_path("fixed.rom");
    let r = common::run(&[
        "patch",
        src.to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
        "--fix-checksum",
    ]);
    assert!(exit_is(&r, 0), "fix-checksum exits 0: {:?}", r);
    assert!(out.is_file());
    assert!(rom_is_valid(&out), "fixed ROM checks out clean");
}

/// TDP within the envelope but above the ROM's own configured limit
/// still patches (with a warning) - the SMC clamps; not a refusal.
#[test]
fn patch_above_configured_limit_warns_but_applies() {
    if common::try_rom(RX570).is_none() {
        eprintln!("skipped: sample ROM not available");
        return;
    }
    let src = work_rom(RX570);
    let out = common::temp_path("over-config.rom");
    let r = common::run(&[
        "patch",
        src.to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
        "--pp-tdp",
        "135",
    ]);
    assert!(
        exit_is(&r, 0),
        "135 W (above 125 W configured) still applies"
    );
    assert!(
        r.0.contains("max power delivery") || r.0.contains("configured limit"),
        "warning mentions the clamp: {}",
        r.0
    );
}
