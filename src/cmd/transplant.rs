//! `transplant` subcommand: transplant PCI ROM images (Legacy/EFI)
//! between VBIOS files with safety guardrails.

use std::fs;
use std::path::Path;
use std::process::ExitCode;

use crate::cli::Command;
use crate::rom::pci::walk_pci_images;
use crate::rom::reader::Reader;
use crate::rom::types::PciImage;

use super::{EXIT_ERROR, EXIT_OK, EXIT_WARNINGS};

// Small helpers

/// Human-readable label for a PCI code type.
fn code_type_label(code_type: u8) -> &'static str {
    if code_type == 0x03 { "EFI" } else { "Legacy" }
}

/// Read a ROM file from disk, producing a descriptive error on failure.
fn read_rom_file(path: &Path, label: &str) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|e| format!("cannot read {label} ROM '{}': {e}", path.display()))
}

/// Parse the PCI option-ROM image chain from raw bytes.
fn parse_pci_chain(data: &[u8], label: &str) -> Result<Vec<PciImage>, String> {
    let r = Reader::new(data);
    walk_pci_images(&r).map_err(|e| format!("cannot parse {label} PCI chain: {e}"))
}

// ATOM signature validation

/// Validates that a Legacy image contains a valid ATOM BIOS signature.
/// Returns Ok(()) if valid, Err(message) if invalid.
fn validate_atom_signature(data: &[u8], img: &PciImage, label: &str) -> Result<(), String> {
    if img.code_type != 0x00 {
        return Ok(()); // Only validate Legacy images
    }
    let r = Reader::new(data);
    let atom_ptr_field = img.file_offset + 0x48;
    if atom_ptr_field + 2 > r.len() {
        return Err(format!(
            "{label} Legacy image too small to contain ATOM pointer"
        ));
    }
    let atom_offset = r
        .u16(atom_ptr_field)
        .map_err(|e| format!("{label} cannot read ATOM pointer: {e}"))?;
    let abs = img.file_offset + atom_offset as usize;
    if abs + 8 > r.len() {
        return Err(format!("{label} ATOM pointer points past end of file"));
    }
    let sig = r
        .bytes(abs + 4, 4)
        .map_err(|e| format!("{label} cannot read ATOM signature: {e}"))?;
    if sig != b"ATOM" {
        return Err(format!(
            "{label} Legacy image has no valid ATOM signature (found {:?} at 0x{:X})",
            sig,
            abs + 4,
        ));
    }
    Ok(())
}

// Types

/// Mode of transplantation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransplantMode {
    /// Transplant EFI image only.
    Efi,
    /// Transplant Legacy image only.
    Legacy,
    /// Transplant both Legacy and EFI images.
    Both,
}

/// Result of validating a transplant operation.
struct TransplantPlan {
    target_chain: Vec<PciImage>,
    donor_chain: Vec<PciImage>,
    target_legacy: Option<PciImage>,
    source_legacy: Option<PciImage>,
    target_efi: Option<PciImage>,
    source_efi: Option<PciImage>,
    warnings: Vec<String>,
}

/// Arguments for validating a transplant operation.
struct TransplantArgs<'a> {
    target_data: &'a [u8],
    donor_data: &'a [u8],
    target_chain: &'a [PciImage],
    donor_chain: &'a [PciImage],
    mode: TransplantMode,
    target_index: Option<usize>,
    donor_index: Option<usize>,
    force: bool,
}

/// Find target and donor images for a given code type, collecting errors.
fn find_image_pair(
    target_chain: &[PciImage],
    donor_chain: &[PciImage],
    code_type: u8,
    target_index: Option<usize>,
    donor_index: Option<usize>,
    all_errors: &mut Vec<String>,
) -> (Option<PciImage>, Option<PciImage>) {
    let t = find_image(target_chain, code_type, target_index);
    let d = find_image(donor_chain, code_type, donor_index);
    match (t, d) {
        (Ok(t), Ok(d)) => (Some(t.clone()), Some(d.clone())),
        (Err(e), _) | (_, Err(e)) => {
            all_errors.push(e);
            (None, None)
        }
    }
}

// Validation helpers

/// Validate a single image pair (target vs donor) for size, device/vendor ID,
/// zero-size, and ATOM signature. Returns `Ok(())` or a list of errors.
fn validate_image_pair(
    target: &PciImage,
    donor: &PciImage,
    donor_data: &[u8],
    target_data: &[u8],
    force: bool,
    label: &str,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if donor.declared_size_bytes > target.declared_size_bytes {
        errors.push(format!(
            "donor {label} ({} bytes) is larger than target {label} ({} bytes); \
             cannot expand without corrupting the PCI ROM chain",
            donor.declared_size_bytes, target.declared_size_bytes,
        ));
    }
    if donor.device_id != target.device_id && !force {
        warnings.push(format!(
            "{label} device ID mismatch: donor=0x{:04X}, target=0x{:04X} (use --force to override)",
            donor.device_id, target.device_id,
        ));
    }
    if donor.vendor_id != target.vendor_id && !force {
        warnings.push(format!(
            "{label} vendor ID mismatch: donor=0x{:04X}, target=0x{:04X}",
            donor.vendor_id, target.vendor_id,
        ));
    }
    if donor.declared_size_bytes == 0 {
        errors.push(format!("donor {label} image has zero declared size"));
    }
    if let Err(e) = validate_atom_signature(donor_data, donor, "donor") {
        errors.push(e);
    }
    if let Err(e) = validate_atom_signature(target_data, target, "target") {
        errors.push(e);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        errors.extend(warnings);
        Err(errors)
    }
}

// Core logic

/// Entry point for the `transplant` subcommand.
pub fn run(cmd: Command) -> ExitCode {
    let (target, from, out, efi, legacy, both, target_index, donor_index, dry_run, force) =
        match &cmd {
            Command::Transplant {
                target,
                from,
                out,
                efi,
                legacy,
                both,
                target_index,
                donor_index,
                dry_run,
                force,
            } => (
                target.as_path(),
                from.as_path(),
                out.as_path(),
                *efi,
                *legacy,
                *both,
                *target_index,
                *donor_index,
                *dry_run,
                *force,
            ),
            _ => unreachable!(),
        };

    let mode = if efi {
        TransplantMode::Efi
    } else if legacy {
        TransplantMode::Legacy
    } else if both {
        TransplantMode::Both
    } else {
        eprintln!("error: specify --efi, --legacy, or --both");
        return ExitCode::from(EXIT_ERROR);
    };

    let target_data = match read_rom_file(target, "target") {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(EXIT_ERROR);
        }
    };
    let donor_data = match read_rom_file(from, "donor") {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(EXIT_ERROR);
        }
    };

    let target_chain = match parse_pci_chain(&target_data, "target") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(EXIT_ERROR);
        }
    };
    let donor_chain = match parse_pci_chain(&donor_data, "donor") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(EXIT_ERROR);
        }
    };

    if target_chain.is_empty() {
        eprintln!("error: target ROM has no PCI option ROM images");
        return ExitCode::from(EXIT_ERROR);
    }
    if donor_chain.is_empty() {
        eprintln!("error: donor ROM has no PCI option ROM images");
        return ExitCode::from(EXIT_ERROR);
    }

    let args = TransplantArgs {
        target_data: &target_data,
        donor_data: &donor_data,
        target_chain: &target_chain,
        donor_chain: &donor_chain,
        mode,
        target_index,
        donor_index,
        force,
    };
    match validate_transplant(&args) {
        Ok(plan) => {
            if dry_run {
                print_plan(&plan, mode, target, from, out, target_data.len());
                ExitCode::from(EXIT_OK)
            } else {
                execute_transplant(&target_data, &donor_data, &plan, mode, out)
            }
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("error: {e}");
            }
            ExitCode::from(EXIT_ERROR)
        }
    }
}

/// Find an image in the chain by code type, optionally at a specific index.
fn find_image(
    chain: &[PciImage],
    code_type: u8,
    index: Option<usize>,
) -> Result<&PciImage, String> {
    let label = code_type_label(code_type);
    if let Some(idx) = index {
        let img = chain.get(idx).ok_or_else(|| {
            format!(
                "index {idx} out of range (chain has {} images)",
                chain.len()
            )
        })?;
        if img.code_type != code_type {
            return Err(format!(
                "index {idx} is {} (code_type=0x{:02X}), expected {label} (code_type=0x{:02X})",
                img.code_type_name, img.code_type, code_type,
            ));
        }
        Ok(img)
    } else {
        chain
            .iter()
            .find(|img| img.code_type == code_type)
            .ok_or_else(|| format!("no {label} image found in chain"))
    }
}

/// Validate a transplant operation and build a plan.
fn validate_transplant(args: &TransplantArgs<'_>) -> Result<TransplantPlan, Vec<String>> {
    let mut all_errors = Vec::new();

    let need_legacy = args.mode != TransplantMode::Efi;
    let need_efi = args.mode != TransplantMode::Legacy;

    let (target_legacy, source_legacy) = if need_legacy {
        let (t, d) = find_image_pair(
            args.target_chain,
            args.donor_chain,
            0x00,
            args.target_index,
            args.donor_index,
            &mut all_errors,
        );
        if !all_errors.is_empty() {
            return Err(all_errors);
        }
        (t, d)
    } else {
        (None, None)
    };

    let (target_efi, source_efi) = if need_efi {
        let (t, d) = find_image_pair(
            args.target_chain,
            args.donor_chain,
            0x03,
            args.target_index,
            args.donor_index,
            &mut all_errors,
        );
        if !all_errors.is_empty() {
            return Err(all_errors);
        }
        (t, d)
    } else {
        (None, None)
    };

    if let (Some(t), Some(d)) = (&target_legacy, &source_legacy) {
        match validate_image_pair(
            t,
            d,
            args.donor_data,
            args.target_data,
            args.force,
            "Legacy",
        ) {
            Ok(()) => {}
            Err(errs) => all_errors.extend(errs),
        }
    }
    if let (Some(t), Some(d)) = (&target_efi, &source_efi) {
        match validate_image_pair(t, d, args.donor_data, args.target_data, args.force, "EFI") {
            Ok(()) => {}
            Err(errs) => all_errors.extend(errs),
        }
    }

    if !all_errors.is_empty() {
        return Err(all_errors);
    }

    Ok(TransplantPlan {
        target_chain: args.target_chain.to_vec(),
        donor_chain: args.donor_chain.to_vec(),
        target_legacy,
        source_legacy,
        target_efi,
        source_efi,
        warnings: Vec::new(),
    })
}

// Print helpers (dry-run)

/// Print the size diff and device-ID status for one image pair.
fn print_image_status(target: &PciImage, donor: &PciImage, label: &str) {
    let size_diff = donor.declared_size_bytes as i64 - target.declared_size_bytes as i64;
    println!(
        "  {label}: donor[{}] ({}B) -> target[{}] ({}B) diff={}B",
        donor.index, donor.declared_size_bytes, target.index, target.declared_size_bytes, size_diff,
    );
    if size_diff < 0 {
        println!("    (donor smaller, will pad with 0xFF)");
    } else if size_diff == 0 {
        println!("    (exact match)");
    }
    if donor.device_id == target.device_id {
        println!("  {label} device IDs match (0x{:04X})", donor.device_id);
    } else {
        println!(
            "  WARNING: {label} device IDs differ (donor=0x{:04X}, target=0x{:04X})",
            donor.device_id, target.device_id,
        );
    }
}

/// Print the transplant plan (dry-run mode).
fn print_plan(
    plan: &TransplantPlan,
    mode: TransplantMode,
    target_path: &Path,
    donor_path: &Path,
    out_path: &Path,
    target_size: usize,
) {
    println!("=== Transplant Plan ===\n");

    println!("Target: {}", target_path.display());
    for img in &plan.target_chain {
        println!(
            "  [{}] {} @ 0x{:05X} ({}B) dev=0x{:04X}",
            img.index, img.code_type_name, img.file_offset, img.declared_size_bytes, img.device_id,
        );
    }

    println!("\nDonor: {}", donor_path.display());
    for img in &plan.donor_chain {
        println!(
            "  [{}] {} @ 0x{:05X} ({}B) dev=0x{:04X}",
            img.index, img.code_type_name, img.file_offset, img.declared_size_bytes, img.device_id,
        );
    }

    let op_str = match mode {
        TransplantMode::Efi => "--efi",
        TransplantMode::Legacy => "--legacy",
        TransplantMode::Both => "--both",
    };
    println!("\nOperation: {op_str}");

    if let (Some(t), Some(d)) = (&plan.target_legacy, &plan.source_legacy) {
        print_image_status(t, d, "Legacy");
    }
    if let (Some(t), Some(d)) = (&plan.target_efi, &plan.source_efi) {
        print_image_status(t, d, "EFI");
    }

    println!("\nGuardrails:");
    if let (Some(t), Some(d)) = (&plan.target_legacy, &plan.source_legacy) {
        if d.device_id == t.device_id {
            println!("  Legacy device IDs match (0x{:04X})", d.device_id);
        } else {
            println!(
                "  WARNING: Legacy device IDs differ (donor=0x{:04X}, target=0x{:04X})",
                d.device_id, t.device_id,
            );
        }
    }
    if let (Some(t), Some(d)) = (&plan.target_efi, &plan.source_efi) {
        if d.device_id == t.device_id {
            println!("  EFI device IDs match (0x{:04X})", d.device_id);
        } else {
            println!(
                "  WARNING: EFI device IDs differ (donor=0x{:04X}, target=0x{:04X})",
                d.device_id, t.device_id,
            );
        }
    }

    if !plan.warnings.is_empty() {
        for w in &plan.warnings {
            println!("  WARNING: {w}");
        }
    }

    println!("\nOutput: {} ({} bytes)", out_path.display(), target_size);
}

// Execution

/// Transplant one image pair (target + donor), returning a labeled error on
/// failure so the caller can decide whether to abort.
fn transplant_pair(
    output: &mut [u8],
    donor_data: &[u8],
    target_data: &[u8],
    target_img: &PciImage,
    donor_img: &PciImage,
    label: &str,
) -> Result<(), String> {
    let target_indicator = target_data[target_img.pcir_offset + 0x15];
    transplant_image(output, donor_data, target_img, donor_img, target_indicator)
        .map_err(|e| format!("{label} transplant failed: {e}"))
}

/// Execute the transplant operation.
fn execute_transplant(
    target_data: &[u8],
    donor_data: &[u8],
    plan: &TransplantPlan,
    mode: TransplantMode,
    out_path: &Path,
) -> ExitCode {
    let mut output = target_data.to_vec();
    if let (Some(t), Some(d)) = (&plan.target_legacy, &plan.source_legacy)
        && let Err(e) = transplant_pair(&mut output, donor_data, target_data, t, d, "legacy")
    {
        eprintln!("error: {e}");
        return ExitCode::from(EXIT_ERROR);
    }

    if let (Some(t), Some(d)) = (&plan.target_efi, &plan.source_efi)
        && let Err(e) = transplant_pair(&mut output, donor_data, target_data, t, d, "EFI")
    {
        eprintln!("error: {e}");
        return ExitCode::from(EXIT_ERROR);
    }

    // Recalculate legacy checksum if we modified the legacy region
    let mut warnings = Vec::new();
    if mode == TransplantMode::Legacy || mode == TransplantMode::Both {
        match crate::rom::patch::fix_checksum(&mut output) {
            Ok(Some(diff)) => {
                println!(
                    "checksum recalculated: byte at 0x{:04X} changed from 0x{:02X} to 0x{:02X}",
                    diff.offset, diff.old[0], diff.new[0]
                );
            }
            Ok(None) => {
                println!("checksum already valid");
            }
            Err(e) => {
                eprintln!("warning: cannot recalculate checksum: {e}");
                warnings.push(format!("checksum recalculation failed: {e}"));
            }
        }
    }

    if let Err(e) = fs::write(out_path, &output) {
        eprintln!("error: cannot write output '{}': {e}", out_path.display());
        return ExitCode::from(EXIT_ERROR);
    }

    println!("wrote {}", out_path.display());
    println!("NOTE: this tool validates structural consistency only (checksum, PCIR, sizes).");
    println!("      it cannot confirm the result will POST on real hardware.");
    println!("      the final validation is always to flash and test.");

    if warnings.is_empty() {
        ExitCode::from(EXIT_OK)
    } else {
        for w in &warnings {
            eprintln!("warning: {w}");
        }
        ExitCode::from(EXIT_WARNINGS)
    }
}

/// Transplant a single image from donor into output at the target's offset.
/// Returns Err if bounds check fails (fatal -- do not write output).
fn transplant_image(
    output: &mut [u8],
    donor_data: &[u8],
    target_img: &PciImage,
    donor_img: &PciImage,
    target_data_indicator: u8,
) -> Result<(), String> {
    let copy_size = donor_img
        .declared_size_bytes
        .min(target_img.declared_size_bytes);
    let src_start = donor_img.file_offset;
    let src_end = src_start + copy_size;
    let dst_start = target_img.file_offset;
    let dst_end = dst_start + copy_size;

    if src_end > donor_data.len() {
        return Err(format!(
            "donor image extends past end of file (src_end=0x{:X}, file_len=0x{:X})",
            src_end,
            donor_data.len(),
        ));
    }
    if dst_end > output.len() {
        return Err(format!(
            "target image extends past end of file (dst_end=0x{:X}, file_len=0x{:X})",
            dst_end,
            output.len(),
        ));
    }

    output[dst_start..dst_end].copy_from_slice(&donor_data[src_start..src_end]);

    // Preserve the target's original indicator byte.
    output[target_img.pcir_offset + 0x15] = target_data_indicator;

    // Pad with 0xFF if donor is smaller than target.
    if donor_img.declared_size_bytes < target_img.declared_size_bytes {
        let pad_start = dst_start + donor_img.declared_size_bytes;
        let pad_end = dst_start + target_img.declared_size_bytes;
        output[pad_start..pad_end].fill(0xFF);
    }

    Ok(())
}
