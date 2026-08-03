//! `patch` subcommand: build validated ops, apply + verify + atomic write.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::cli::Command;
use crate::cmd;
use crate::rom;
use crate::rom::patch::PatchOp;
use crate::rom::reader::Reader;

/// Parses every `--set-strap`, `--set-strap-reg`, `--retag-strap`,
/// `--timing`, `--pp-*`, `--vram-size-mb` and `--hex` argument of the
/// patch command into ops. Reference-ROM ops (`--clone-ids`,
/// `--import-vram`) are resolved later by [`run`] because they need
/// file I/O.
fn build_ops(cmd: &Command) -> Result<Vec<PatchOp>, String> {
    let Command::Patch {
        set_strap,
        set_strap_reg,
        retag_strap,
        timing,
        pp_sclk,
        pp_mclk,
        pp_vddc,
        pp_tdp,
        hex,
        vram_size_mb,
        i_understand_strap_mismatch,
        import_vram,
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
    for g in timing.chunks_exact(3) {
        ops.push(parse_timing(&g[0], &g[1], &g[2])?);
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
    // Two answers to the same question ("what size does this ROM
    // declare?") is ambiguous by nature: the import brings the donor's
    // whole calibrated table, the manual edit writes only geometry.
    if import_vram.is_some() && vram_size_mb.is_some() {
        return Err(
            "--import-vram and --vram-size-mb are mutually exclusive: the import brings the \
             donor's whole factory-calibrated table, the manual edit writes geometry only"
                .to_string(),
        );
    }
    if let Some(n) = vram_size_mb {
        let size_mb: u32 = cmd::parse_num(n, "VRAM size (MB)")?;
        if size_mb == 0 {
            return Err("--vram-size-mb must be a positive size in MB".to_string());
        }
        ops.push(PatchOp::VramSizeMb {
            size_mb,
            understand: *i_understand_strap_mismatch,
        });
    }
    Ok(ops)
}

/// Parses one `--timing <clock> <field> <value>` group into a
/// [`PatchOp::SetTiming`]. The value is clock cycles, or nanoseconds
/// when it carries an `ns` suffix - then converted to cycles at the
/// given clock. Refused when the field is unknown or the value does not
/// fit the field's bit width (fail fast, before touching the ROM).
fn parse_timing(clock: &str, field: &str, value: &str) -> Result<PatchOp, String> {
    let clock_mhz: u32 = cmd::parse_num(clock, "clock (MHz)")?;
    if clock_mhz == 0 {
        return Err("--timing clock must be positive".to_string());
    }
    let (reg, f) = rom::timings::field_named(field).ok_or_else(|| {
        format!(
            "unknown memory timing field '{field}' (known: {})",
            rom::timings::CORE_TIMINGS.join(", ")
        )
    })?;
    let lower = value.to_ascii_lowercase();
    let cycles = if let Some(v) = lower.strip_suffix("ns") {
        let ns: f64 = v
            .trim()
            .parse()
            .map_err(|_| format!("cannot parse '{value}' as nanoseconds"))?;
        if ns <= 0.0 {
            return Err("--timing value in ns must be positive".to_string());
        }
        (ns * clock_mhz as f64 / 1000.0).round() as u32
    } else {
        let c: i64 = cmd::parse_num(value, "timing value")?;
        if c < 0 {
            return Err("--timing value must be positive".to_string());
        }
        c as u32
    };
    let max = (1u32 << f.width) - 1;
    if cycles > max {
        return Err(format!(
            "--timing {clock_mhz} {field} {cycles} cycles exceeds the {}-bit {field} field \
             of {} (max {max})",
            f.width, reg.name
        ));
    }
    Ok(PatchOp::SetTiming {
        clock_mhz,
        field: f.name,
        cycles,
    })
}

/// Reads a reference ROM and resolves the values `--clone-ids` needs:
/// the device-id (first PCI option ROM image) and the subsystem
/// vendor/device pair of the ATOM header.
fn resolve_clone_ids(ref_path: &Path) -> Result<PatchOp, String> {
    let data =
        fs::read(ref_path).map_err(|e| format!("error reading '{}': {e}", ref_path.display()))?;
    let r = Reader::new(&data);
    let atom_ptr = r
        .u16(0x48)
        .map_err(|e| format!("'{}' is not a valid ROM: {e}", ref_path.display()))?
        as usize;
    if r.bytes(atom_ptr + 4, 4)
        .map_err(|e| format!("'{}' has no ATOM header: {e}", ref_path.display()))?
        != b"ATOM"
    {
        return Err(format!("'{}' has no ATOM header", ref_path.display()));
    }
    let images =
        rom::pci::walk_pci_images(&r).map_err(|e| format!("'{}': {e}", ref_path.display()))?;
    let img = images
        .first()
        .ok_or_else(|| format!("'{}' has no PCI option ROM image", ref_path.display()))?;
    let subsystem_vendor = r
        .u16(atom_ptr + 0x18)
        .map_err(|e| format!("'{}': {e}", ref_path.display()))?;
    let subsystem_device = r
        .u16(atom_ptr + 0x1A)
        .map_err(|e| format!("'{}': {e}", ref_path.display()))?;
    Ok(PatchOp::CloneIds {
        device: img.device_id,
        subsystem_vendor,
        subsystem_device,
        ref_device_id: img.device_id,
    })
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
    timing: Vec<String>,
    pp_sclk: Vec<String>,
    pp_mclk: Vec<String>,
    pp_vddc: Vec<String>,
    pp_tdp: Vec<String>,
    hex: Vec<String>,
    clone_ids: Option<PathBuf>,
    import_vram: Option<PathBuf>,
    vram_size_mb: Option<String>,
    i_understand_strap_mismatch: bool,
) -> ExitCode {
    let cmd = Command::Patch {
        rom: rom_path.to_path_buf(),
        out: out_path.to_path_buf(),
        dry_run,
        fix_checksum,
        set_strap,
        set_strap_reg,
        retag_strap,
        timing,
        pp_sclk,
        pp_mclk,
        pp_vddc,
        pp_tdp,
        hex,
        clone_ids,
        import_vram,
        vram_size_mb,
        i_understand_strap_mismatch,
    };
    let mut ops = match build_ops(&cmd) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(cmd::EXIT_ERROR);
        }
    };
    // Reference-ROM ops need file I/O: resolve them here, before the
    // emptiness check, so e.g. `--import-vram` alone is a valid edit.
    let Command::Patch {
        clone_ids,
        import_vram,
        ..
    } = &cmd
    else {
        unreachable!()
    };
    if let Some(ref_path) = clone_ids {
        match resolve_clone_ids(ref_path) {
            Ok(op) => ops.push(op),
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::from(cmd::EXIT_ERROR);
            }
        }
    }
    if let Some(ref_path) = import_vram {
        let data = match cmd::read_rom(ref_path) {
            Ok(d) => d,
            Err(code) => return code,
        };
        ops.push(PatchOp::ImportVram { donor: data });
    }
    if ops.is_empty() && !fix_checksum {
        eprintln!(
            "error: nothing to do - add an edit (--set-strap, --timing, --pp-*, --hex...) \
             or --fix-checksum"
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
