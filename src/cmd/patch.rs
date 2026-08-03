//! `patch` subcommand: build validated ops, apply + verify + atomic write.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use crate::cli::Command;
use crate::rom;
use crate::{cmd, rom::patch::PatchOp};

/// Parses every `--set-strap`, `--set-strap-reg`, `--retag-strap`,
/// `--pp-*` and `--hex` argument of the patch command into ops.
fn build_ops(cmd: &Command) -> Result<Vec<PatchOp>, String> {
    let Command::Patch {
        set_strap,
        set_strap_reg,
        retag_strap,
        pp_sclk,
        pp_mclk,
        pp_vddc,
        pp_tdp,
        hex,
        ..
    } = cmd
    else {
        unreachable!()
    };
    let mut ops = Vec::new();
    // Every --set-strap consumes exactly 3 values.
    for g in set_strap.chunks_exact(3) {
        ops.push(PatchOp::SetStrap {
            clock_mhz: cmd::parse_num(&g[0], "clock (MHz)")?,
            reg: cmd::parse_num(&g[1], "register index")?,
            value: cmd::parse_u32_hex(&g[2], "--set-strap")?,
        });
    }
    for g in set_strap_reg.chunks_exact(2) {
        ops.push(PatchOp::SetStrapReg {
            reg_offset: cmd::parse_u32_hex(&g[0], "--set-strap-reg")?,
            value: cmd::parse_u32_hex(&g[1], "--set-strap-reg")?,
        });
    }
    for g in retag_strap.chunks_exact(2) {
        ops.push(PatchOp::RetagStrap {
            clock_mhz: cmd::parse_num(&g[0], "clock (MHz)")?,
            new_clock_mhz: cmd::parse_num(&g[1], "new clock (MHz)")?,
        });
    }
    for g in pp_sclk.chunks_exact(2) {
        ops.push(PatchOp::PpSclk {
            level: cmd::parse_num(&g[0], "SCLK level")?,
            mhz: cmd::parse_num(&g[1], "clock (MHz)")?,
        });
    }
    for g in pp_mclk.chunks_exact(2) {
        ops.push(PatchOp::PpMclk {
            level: cmd::parse_num(&g[0], "MCLK level")?,
            mhz: cmd::parse_num(&g[1], "clock (MHz)")?,
        });
    }
    for g in pp_vddc.chunks_exact(2) {
        ops.push(PatchOp::PpVddc {
            index: cmd::parse_num(&g[0], "LUT index")?,
            mv: cmd::parse_num(&g[1], "voltage (mV)")?,
        });
    }
    for w in pp_tdp {
        ops.push(PatchOp::PpTdp {
            watts: cmd::parse_num(w, "TDP (W)")?,
        });
    }
    for g in hex.chunks_exact(2) {
        let bytes = cmd::parse_hex_bytes(&g[1], "--hex")?;
        if bytes.is_empty() {
            return Err("--hex needs at least one byte".to_string());
        }
        ops.push(PatchOp::Hex {
            offset: cmd::parse_u32_hex(&g[0], "--hex offset")? as usize,
            bytes,
        });
    }
    Ok(ops)
}

#[allow(clippy::too_many_arguments)]
pub fn run(
    rom_path: &Path,
    out_path: &Path,
    dry_run: bool,
    fix_checksum: bool,
    set_strap: Vec<String>,
    set_strap_reg: Vec<String>,
    retag_strap: Vec<String>,
    pp_sclk: Vec<String>,
    pp_mclk: Vec<String>,
    pp_vddc: Vec<String>,
    pp_tdp: Vec<String>,
    hex: Vec<String>,
) -> ExitCode {
    let cmd = Command::Patch {
        rom: rom_path.to_path_buf(),
        out: out_path.to_path_buf(),
        dry_run,
        fix_checksum,
        set_strap,
        set_strap_reg,
        retag_strap,
        pp_sclk,
        pp_mclk,
        pp_vddc,
        pp_tdp,
        hex,
    };
    let ops = match build_ops(&cmd) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(cmd::EXIT_ERROR);
        }
    };
    if ops.is_empty() && !fix_checksum {
        eprintln!(
            "error: nothing to do - add an edit (--set-strap, --pp-*, --hex...) or --fix-checksum"
        );
        return ExitCode::from(cmd::EXIT_ERROR);
    }

    // Never in place (canonical paths, and same (dev, inode) so hard
    // links to the source cannot be silently overwritten either).
    if same_file(rom_path, out_path) {
        eprintln!(
            "error: --out must be a different file than the source ROM (never patch in place)"
        );
        return ExitCode::from(cmd::EXIT_ERROR);
    }

    let mut data = match cmd::read_rom(rom_path) {
        Ok(d) => d,
        Err(code) => return code,
    };

    // Input checksum: refuse to patch a corrupt image unless repairing.
    let input_valid = match rom::parse_rom(rom_path) {
        Ok(r) => r.header.checksum_valid,
        Err(e) => {
            eprintln!("error reading '{}': {e:#}", rom_path.display());
            return ExitCode::from(cmd::EXIT_ERROR);
        }
    };
    if !input_valid && !fix_checksum {
        eprintln!(
            "error: input ROM checksum is invalid - refusing to patch a corrupt image \
             (run with --fix-checksum to repair it)"
        );
        return ExitCode::from(cmd::EXIT_ERROR);
    }

    // Repair path: --fix-checksum repairs the input first, so the edits
    // are validated against a checksum-clean image.
    let mut pre_fix: Option<rom::patch::Diff> = None;
    if !input_valid {
        match rom::patch::fix_checksum(&mut data) {
            Ok(d) => pre_fix = d,
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::from(cmd::EXIT_ERROR);
            }
        }
    }

    let report = match rom::patch::apply_ops(&data, &ops, pre_fix.is_some()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e:#}");
            return ExitCode::from(cmd::EXIT_ERROR);
        }
    };

    let mut patched = data.clone();
    rom::patch::apply_diffs(&mut patched, &report.diffs);

    // Recompute the legacy checksum over the edited image.
    let checksum_diff = match rom::patch::fix_checksum(&mut patched) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(cmd::EXIT_ERROR);
        }
    };

    // Report.
    println!("patch plan for '{}':", rom_path.display());
    if let Some(d) = &pre_fix {
        println!(
            "  0x{:06X}  {} -> {}  ({})",
            d.offset,
            rom::patch::Diff::hex_pairs(&d.old),
            rom::patch::Diff::hex_pairs(&d.new),
            d.label
        );
    }
    for d in &report.diffs {
        println!(
            "  0x{:06X}  {} -> {}  ({})",
            d.offset,
            rom::patch::Diff::hex_pairs(&d.old),
            rom::patch::Diff::hex_pairs(&d.new),
            d.label
        );
    }
    if let Some(cd) = &checksum_diff {
        println!(
            "  0x{:06X}  {} -> {}  ({})",
            cd.offset,
            rom::patch::Diff::hex_pairs(&cd.old),
            rom::patch::Diff::hex_pairs(&cd.new),
            cd.label
        );
    } else if pre_fix.is_none() {
        println!("  (checksum already valid - no fix needed)");
    }
    let overlap_notes = rom::patch::overlapping_diffs(&report.diffs);
    for w in &report.warnings {
        println!("  warning: {w}");
    }
    for w in &overlap_notes {
        println!("  warning: {w}");
    }
    let total = report.diffs.len()
        + overlap_notes.len()
        + pre_fix.is_some() as usize
        + checksum_diff.is_some() as usize;
    println!(
        "  {} edit(s), {} warning(s)",
        total,
        report.warnings.len() + overlap_notes.len()
    );

    // Structural verification before writing: nothing but the reported
    // diffs may have touched layout-defining bytes.
    let mut all_diffs = report.diffs.clone();
    if let Some(d) = &checksum_diff {
        all_diffs.push(d.clone());
    }
    match rom::patch::verify_structural(&data, &patched, &all_diffs) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("error: verification failed, refusing to write: {e}");
            return ExitCode::from(cmd::EXIT_ERROR);
        }
    }

    // Full re-parse + disasm sweep: refuse on any structural failure.
    match rom::patch::verify(&patched) {
        Ok(verify_warnings) => {
            for w in &verify_warnings {
                println!("  post-patch note: {w}");
            }
        }
        Err(e) => {
            eprintln!("error: verification failed, refusing to write: {e}");
            return ExitCode::from(cmd::EXIT_ERROR);
        }
    }

    if dry_run {
        println!("dry run - nothing written");
        return ExitCode::from(cmd::EXIT_OK);
    }

    match write_atomic(out_path, &patched) {
        Ok(()) => {
            println!(
                "wrote {} ({} bytes, checksum recomputed)",
                out_path.display(),
                patched.len()
            );
            ExitCode::from(cmd::EXIT_OK)
        }
        Err(e) => {
            eprintln!("error writing '{}': {e}", out_path.display());
            ExitCode::from(cmd::EXIT_ERROR)
        }
    }
}

/// True when both paths refer to the same file: same underlying file
/// object (catches hard links) or equal canonical paths.
fn same_file(a: &Path, b: &Path) -> bool {
    if same_file::is_same_file(a, b).unwrap_or(false) {
        return true;
    }
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => a == b,
    }
}

/// Writes to a temp file in the same directory and renames it over the
/// target: the destination either has the complete new content or the
/// old one - never a half-written ROM (e.g. after a power cut). The
/// data is flushed to disk (`sync_all`) before the rename so a crash
/// cannot leave a truncated file behind under the final name.
fn write_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "out".to_string());
    let tmp = path.with_file_name(format!(".{name}.{}.tmp", std::process::id()));
    let result = (|| -> std::io::Result<()> {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(data)?;
        f.sync_all()?;
        drop(f);
        fs::rename(&tmp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}
