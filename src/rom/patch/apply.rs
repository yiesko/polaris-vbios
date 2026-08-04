use anyhow::{Context, Result, bail};

use super::limits::HardLimitRec;
use super::limits::find_strap;
use super::map::{RomMap, map_rom, overlaps_layout, structure_contains};
use super::types::{Diff, PatchOp, PatchReport};
use crate::compare_util;
use crate::rom::header;
use crate::rom::locate;
use crate::rom::pci;
use crate::rom::reader::Reader;
use crate::rom::validate;
use crate::rom::vram;

/// Applies a list of ops to a ROM image in memory. Every edit is
/// validated first: out-of-range, no-op, clock above the ROM's own
/// hard limit, VDDC above the die maximum - any of these refuse the
/// whole patch (nothing applied). When `allow_invalid_checksum` is set
/// (the `--fix-checksum` repair path), the input checksum gate is
/// skipped - the caller must have repaired the image first.
pub fn apply_ops(
    data: &[u8],
    ops: &[PatchOp],
    allow_invalid_checksum: bool,
) -> Result<PatchReport> {
    let map = map_rom(data)?;
    let r = Reader::new(data);

    // The input must be checksum-valid: patching a corrupt image would
    // produce a "valid-looking" but broken ROM.
    let (_, checked_sum) = super::checksum::checksum_state(data)?;
    if checked_sum != 0 && !allow_invalid_checksum {
        bail!(
            "input ROM checksum is invalid (sum 0x{checked_sum:02X}); refusing to patch a \
             corrupt image (use --fix-checksum to repair it first)"
        );
    }

    let hard_limits = super::limits::read_hard_limits(&r, map.pp_off);
    let mut report = PatchReport::default();
    for op in ops {
        let changed = apply_one(&map, data, &hard_limits, op, &mut report)
            .with_context(|| format!("op rejected: {op:?}"))?;
        if !changed {
            bail!("no-op: {op:?} would not change anything (value already identical)");
        }
    }
    Ok(report)
}

/// Refuses `--hex` writes that would damage the boot area, a structure
/// header or the master tables; warns when they target a parsed
/// structure's data or sit outside the checksum-covered region.
fn guard_hex(
    map: &RomMap,
    r: &Reader,
    report: &mut PatchReport,
    offset: usize,
    len: usize,
) -> Result<()> {
    if offset + len > r.len() {
        bail!(
            "--hex at 0x{offset:X} + {len} bytes runs past the end of the file ({} bytes)",
            r.len()
        );
    }
    if let Some((start, end)) = overlaps_layout(map, r, offset, len) {
        bail!(
            "refusing --hex at 0x{offset:X}: overlaps protected layout area \
             [0x{start:X}, 0x{end:X}) (boot area, BIOS data area, ATOM header, master tables, \
             PCI data structure or a sub-table header)"
        );
    }
    if offset >= map.declared_bytes {
        report.warnings.push(format!(
            "--hex at 0x{offset:X} is outside the checksum-covered legacy region (0x{:X} \
             bytes): writes here are not covered by the recomputed checksum and may target the \
             EFI/GOP image",
            map.declared_bytes
        ));
    }
    if structure_contains(map, r, offset, len) {
        report.warnings.push(format!(
            "--hex at 0x{offset:X} overlaps a known parsed structure (PowerPlay/VRAM/straps) - \
             only use this after understanding what lives there"
        ));
    }
    Ok(())
}

/// Records a byte-range edit; refuses no-ops. Returns false when the
/// value is already identical (the caller decides whether that is a
/// hard error).
fn push_diff(
    r: &Reader,
    report: &mut PatchReport,
    offset: usize,
    new: &[u8],
    label: &str,
) -> Result<bool> {
    if offset + new.len() > r.len() {
        bail!(
            "patch offset 0x{offset:X} + {} bytes is out of the file",
            new.len()
        );
    }
    let old = r.bytes(offset, new.len())?;
    if old == new {
        return Ok(false);
    }
    report.diffs.push(Diff {
        offset,
        old: old.to_vec(),
        new: new.to_vec(),
        label: label.to_string(),
    });
    Ok(true)
}

#[allow(clippy::too_many_arguments)]
fn apply_one(
    map: &RomMap,
    data: &[u8],
    hard_limits: &[HardLimitRec],
    op: &PatchOp,
    report: &mut PatchReport,
) -> Result<bool> {
    let r = Reader::new(data);
    let vram_off = || {
        map.vram_off
            .ok_or_else(|| anyhow::anyhow!("ROM has no VRAM_Info table"))
    };
    let pp_off = || {
        map.pp_off
            .ok_or_else(|| anyhow::anyhow!("ROM has no PowerPlayInfo table"))
    };
    match op {
        PatchOp::SetStrap {
            clock_mhz,
            reg,
            value,
        } => {
            let vram_off = vram_off()?;
            let (_, block_size) = locate::strap_region(&r, vram_off)
                .ok_or_else(|| anyhow::anyhow!("ROM has no memory strap table"))?;
            // The strap block is `clock u32 + regs u32...`; a register
            // position past the block would write into the next strap's
            // clock field.
            let n_vals = (block_size / 4).saturating_sub(1);
            if *reg >= n_vals {
                bail!(
                    "strap {clock_mhz} MHz register {reg} is outside the strap block: a \
                     {block_size}-byte block holds {n_vals} register slots (refusing)"
                );
            }
            let idx = find_strap(&r, vram_off, *clock_mhz)?;
            let off = locate::strap_value(&r, vram_off, idx, *reg).ok_or_else(|| {
                anyhow::anyhow!(
                    "strap {clock_mhz} MHz has no register position {reg} \
                     (it has {} registers)",
                    super::limits::n_regs(&r, vram_off)
                )
            })?;
            let reg_off = locate::strap_reg_index(&r, vram_off, *reg)
                .and_then(|o| r.u16(o).ok())
                .unwrap_or(0);
            let old = r.u32(off)?;
            push_diff(
                &r,
                report,
                off,
                &value.to_le_bytes(),
                &format!(
                    "strap {clock_mhz} MHz reg {reg} (MC 0x{reg_off:X}): 0x{old:08X} -> \
                     0x{value:08X}"
                ),
            )
        }
        PatchOp::SetStrapReg { reg_offset, value } => {
            if *reg_offset > u16::MAX as u32 {
                bail!(
                    "MC register offset 0x{reg_offset:X} is out of range (max 0xFFFF) - \
                     refusing to guess a truncated register"
                );
            }
            let vram_off = vram_off()?;
            let n = super::limits::n_regs(&r, vram_off);
            let mut found = None;
            for i in 0..n {
                let o = locate::strap_reg_index(&r, vram_off, i)
                    .ok_or_else(|| anyhow::anyhow!("ROM has no strap register index table"))?;
                if r.u16(o)? == *reg_offset as u16 {
                    found = Some(i);
                    break;
                }
            }
            let reg = found.ok_or_else(|| {
                anyhow::anyhow!(
                    "MC register 0x{reg_offset:X} is not in the strap register index table"
                )
            })?;
            let (data_start, block_size) = locate::strap_region(&r, vram_off)
                .ok_or_else(|| anyhow::anyhow!("ROM has no memory strap table"))?;
            let n_vals = (block_size / 4).saturating_sub(1);
            if reg >= n_vals {
                bail!(
                    "MC register 0x{reg_offset:X} (index {reg}) is outside the strap block: a \
                     {block_size}-byte block holds {n_vals} register slots (refusing)"
                );
            }
            let mut applied = 0;
            for idx in 0..super::limits::strap_count(&r, data_start, block_size) {
                let off = locate::strap_value(&r, vram_off, idx, reg)
                    .ok_or_else(|| anyhow::anyhow!("strap block {idx} has no register {reg}"))?;
                let old = r.u32(off)?;
                if push_diff(
                    &r,
                    report,
                    off,
                    &value.to_le_bytes(),
                    &format!(
                        "MC register 0x{reg_offset:X} (block {idx}): 0x{old:08X} -> \
                         0x{value:08X}"
                    ),
                )? {
                    applied += 1;
                }
            }
            if applied == 0 {
                bail!(
                    "MC register 0x{reg_offset:X} already holds 0x{value:08X} in every strap \
                     (no-op)"
                );
            }
            Ok(true)
        }
        PatchOp::SetTiming {
            clock_mhz,
            field,
            cycles,
        } => {
            let vram_off = vram_off()?;
            // The field was resolved at parse time; re-resolve here so
            // the op stays self-contained.
            let (reg, f) = crate::rom::timings::field_named(field)
                .ok_or_else(|| anyhow::anyhow!("unknown memory timing field '{field}'"))?;
            let n = super::limits::n_regs(&r, vram_off);
            let mut slot = None;
            for i in 0..n {
                let o = locate::strap_reg_index(&r, vram_off, i)
                    .ok_or_else(|| anyhow::anyhow!("ROM has no strap register index table"))?;
                if r.u16(o)? == reg.index {
                    slot = Some(i);
                    break;
                }
            }
            let slot = slot.ok_or_else(|| {
                anyhow::anyhow!(
                    "MC register 0x{:X} ({}) is not in the strap register index table",
                    reg.index,
                    reg.name
                )
            })?;
            let (data_start, block_size) = locate::strap_region(&r, vram_off)
                .ok_or_else(|| anyhow::anyhow!("ROM has no memory strap table"))?;
            let mask = (1u32 << f.width) - 1;
            let want = super::limits::centi_mhz(*clock_mhz)?;
            let mut applied = 0;
            for idx in 0..super::limits::strap_count(&r, data_start, block_size) {
                let clock_raw = r.u32(data_start + idx * block_size)?;
                if clock_raw & 0xFF_FFFF != want {
                    continue;
                }
                let off = locate::strap_value(&r, vram_off, idx, slot)
                    .ok_or_else(|| anyhow::anyhow!("strap block {idx} has no register {slot}"))?;
                let old = r.u32(off)?;
                let new = (old & !(mask << f.offset)) | (*cycles << f.offset);
                if push_diff(
                    &r,
                    report,
                    off,
                    &new.to_le_bytes(),
                    &format!(
                        "strap {clock_mhz} MHz {} (0x{:X}): {} -> {}",
                        field, reg.index, old, new
                    ),
                )? {
                    applied += 1;
                }
            }
            if applied == 0 {
                let clocks = (0..super::limits::strap_count(&r, data_start, block_size))
                    .filter_map(|i| r.u32(data_start + i * block_size).ok())
                    .map(|raw| format!("{}", (raw & 0xFF_FFFF) / 100))
                    .collect::<Vec<_>>()
                    .join(", ");
                if clocks.is_empty() {
                    bail!("no memory strap found in this ROM");
                }
                let old = if clocks.contains(&clock_mhz.to_string()) {
                    format!(
                        "{} already holds {cycles} cycles in every {clock_mhz} MHz strap",
                        field
                    )
                } else {
                    format!("no strap with clock {clock_mhz} MHz found (available: {clocks})")
                };
                bail!("{old} (no-op)");
            }
            Ok(true)
        }
        PatchOp::RetagStrap {
            clock_mhz,
            new_clock_mhz,
        } => {
            let new_c = super::limits::centi_mhz(*new_clock_mhz)?;
            let vram_off = vram_off()?;
            super::limits::guard_clock(hard_limits, *new_clock_mhz as f64, |e| e.mclk_mhz, "MCLK")?;
            // The memory controller only trains the straps the vendor
            // shipped; retagging above the highest of them programs a
            // clock the MC ucode has never trained for that device.
            if let Some(max_clock) = super::limits::max_strap_clock(&r, vram_off)
                && *new_clock_mhz > max_clock
            {
                bail!(
                    "cannot retag strap {clock_mhz} MHz to {new_clock_mhz} MHz: the memory \
                     controller only trains straps up to {max_clock} MHz in this ROM (refusing)"
                );
            }
            let idx = find_strap(&r, vram_off, *clock_mhz)?;
            let off = locate::strap_clock_field(&r, vram_off, idx)
                .ok_or_else(|| anyhow::anyhow!("ROM has no memory strap table"))?;
            let old_raw = r.u32(off)?;
            let new_raw = (old_raw & 0xFF00_0000) | new_c;
            // Warn when the new clock has no MCLK DPM level to pair
            // with - the strap clock should match a trained DPM clock.
            if let Some(pp_off) = map.pp_off
                && let Some(mclk_off) = locate::powerplay_mclk_table(&r, pp_off)
                && let Ok(n) = r.u8(mclk_off + 1)
            {
                let dpm_clocks = (0..n)
                    .filter_map(|i| locate::mclk_value(&r, mclk_off, i as usize))
                    .filter_map(|o| r.u32(o).ok())
                    .map(|c| c / 100)
                    .collect::<Vec<_>>();
                if !dpm_clocks.contains(new_clock_mhz) {
                    report.warnings.push(format!(
                        "retagging to {new_clock_mhz} MHz: no MCLK DPM level at that clock \
                         (DPM levels: {}) - the memory controller may not train it",
                        dpm_clocks
                            .iter()
                            .map(|c| c.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
            }
            push_diff(
                &r,
                report,
                off,
                &new_raw.to_le_bytes(),
                &format!("strap {clock_mhz} MHz -> {new_clock_mhz} MHz (retag)"),
            )
        }
        PatchOp::PpSclk { level, mhz } => {
            let pp_off = pp_off()?;
            let sclk_off = locate::powerplay_sclk_table(&r, pp_off)
                .ok_or_else(|| anyhow::anyhow!("ROM has no SCLK DPM table"))?;
            let off = locate::sclk_value(&r, sclk_off, *level)
                .ok_or_else(|| anyhow::anyhow!("SCLK DPM has no level {level}"))?;
            let centi = super::limits::centi_mhz(*mhz)?;
            super::limits::guard_clock(hard_limits, *mhz as f64, |e| e.sclk_mhz, "SCLK")?;
            if !(100..=2500).contains(mhz) {
                report.warnings.push(format!(
                    "SCLK {mhz} MHz is outside the usual 100-2500 MHz range (applied anyway)"
                ));
            }
            let old = r.u32(off)? as f64 / 100.0;
            push_diff(
                &r,
                report,
                off,
                &centi.to_le_bytes(),
                &format!("SCLK DPM level {level}: {old:.0} MHz -> {mhz} MHz"),
            )
        }
        PatchOp::PpMclk { level, mhz } => {
            let pp_off = pp_off()?;
            let mclk_off = locate::powerplay_mclk_table(&r, pp_off)
                .ok_or_else(|| anyhow::anyhow!("ROM has no MCLK DPM table"))?;
            let off = locate::mclk_value(&r, mclk_off, *level)
                .ok_or_else(|| anyhow::anyhow!("MCLK DPM has no level {level}"))?;
            let centi = super::limits::centi_mhz(*mhz)?;
            super::limits::guard_clock(hard_limits, *mhz as f64, |e| e.mclk_mhz, "MCLK")?;
            if !(100..=3000).contains(mhz) {
                report.warnings.push(format!(
                    "MCLK {mhz} MHz is outside the usual 100-3000 MHz range (applied anyway)"
                ));
            }
            // The memory controller only trains the strap clocks the
            // vendor shipped; a DPM clock with no matching strap may
            // never be trained (mirror of the retag DPM-pairing check).
            if let Some(vram_off) = map.vram_off
                && let Some((data_start, block_size)) = locate::strap_region(&r, vram_off)
            {
                let strap_clocks = (0..super::limits::strap_count(&r, data_start, block_size))
                    .filter_map(|i| r.u32(data_start + i * block_size).ok())
                    .map(|raw| (raw & 0xFF_FFFF) / 100)
                    .collect::<Vec<_>>();
                if !strap_clocks.contains(mhz) {
                    report.warnings.push(format!(
                        "MCLK DPM level {level}: {mhz} MHz has no matching memory strap \
                         (strap clocks: {}) - the memory controller may not train it",
                        strap_clocks
                            .iter()
                            .map(|c| c.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
            }
            let old = r.u32(off)? as f64 / 100.0;
            push_diff(
                &r,
                report,
                off,
                &centi.to_le_bytes(),
                &format!("MCLK DPM level {level}: {old:.0} MHz -> {mhz} MHz"),
            )
        }
        PatchOp::PpVddc { index, mv } => {
            let pp_off = pp_off()?;
            let lut_off = locate::powerplay_vddc_lut(&r, pp_off)
                .ok_or_else(|| anyhow::anyhow!("ROM has no VDDC LUT table"))?;
            let off = locate::vddc_lut_entry(&r, lut_off, *index)
                .ok_or_else(|| anyhow::anyhow!("VDDC LUT has no entry {index}"))?;
            if let Some(die_max) = super::limits::die_max_mv(&r, map.mdt)
                && *mv as u32 > die_max
            {
                bail!(
                    "VDDC {mv} mV exceeds the die maximum {die_max} mV declared in \
                     ASIC_ProfilingInfo - refusing (would risk the GPU)"
                );
            }
            if let Some(limit) = hard_limits
                .iter()
                .map(|h| h.vddc_mv)
                .max()
                .filter(|&v| v > 0)
                && *mv > limit
            {
                report.warnings.push(format!(
                    "VDDC {mv} mV exceeds the hard limit of {limit} mV (applied anyway - the \
                     SMC clamps to the safe value)"
                ));
            }
            if !(400..=1600).contains(mv) {
                report.warnings.push(format!(
                    "VDDC {mv} mV is outside the usual 400-1600 mV range (applied anyway)"
                ));
            }
            let old = r.u16(off)?;
            push_diff(
                &r,
                report,
                off,
                &mv.to_le_bytes(),
                &format!("VDDC LUT entry {index}: {old} mV -> {mv} mV"),
            )
        }
        PatchOp::PpTdp { watts } => {
            let pp_off = pp_off()?;
            let pt_off = locate::powerplay_powertune(&r, pp_off)
                .ok_or_else(|| anyhow::anyhow!("ROM has no PowerTune table"))?;
            let off = locate::powertune_tdp(pt_off);
            if !(30..=300).contains(watts) {
                report.warnings.push(format!(
                    "TDP {watts} W is outside the usual 30-300 W range (applied anyway)"
                ));
            }
            // The PowerPlay table declares the highest configurable TDP
            // and the max power delivery the firmware accepts; warn above
            // them (the SMC clamps to the cap, same as the VDDC hard limit).
            let declared_cap = r
                .u16(pt_off + 3)
                .unwrap_or(0)
                .max(r.u16(pt_off + 15).unwrap_or(0));
            if declared_cap > 0 && *watts as u32 > declared_cap as u32 {
                report.warnings.push(format!(
                    "TDP {watts} W exceeds the {declared_cap} W configured limit declared by this ROM \
                     (applied anyway - the SMC clamps to the safe value)"
                ));
            }
            let old = r.u16(off)?;
            push_diff(
                &r,
                report,
                off,
                &watts.to_le_bytes(),
                &format!(
                    "PowerTune TDP: {old} W -> {watts} W ({})",
                    compare_util::pct_delta(old as f64, *watts as f64)
                ),
            )
        }
        PatchOp::Hex { offset, bytes } => {
            guard_hex(map, &r, report, *offset, bytes.len())?;
            push_diff(&r, report, *offset, bytes, "raw hex")
        }
        PatchOp::CloneIds {
            device,
            subsystem_vendor,
            subsystem_device,
            ref_device_id,
        } => {
            let images = pci::walk_pci_images(&r)?;
            let dest_device = images
                .first()
                .map(|img| img.device_id)
                .ok_or_else(|| anyhow::anyhow!("ROM has no PCI option ROM image"))?;
            // Identity cloning is not blocked, but presenting one die
            // as another deserves a loud, non-blocking warning.
            match (
                validate::die_for_device_id(dest_device),
                validate::die_for_device_id(*ref_device_id),
            ) {
                (Some(dest), Some(reference)) if dest.0 != reference.0 => {
                    report.warnings.push(format!(
                        "device-id mismatch: this ROM claims a {} die but the reference claims \
                         {} - the cloned id will present the reference's silicon",
                        dest.0, reference.0
                    ));
                }
                (None, _) | (_, None) => report.warnings.push(format!(
                    "cannot compare dies (unknown device-id 0x{dest_device:04X}/\
                     0x{ref_device_id:04X}) - cloning ids anyway"
                )),
                _ => {}
            }
            let mut changed = false;
            for img in &images {
                if !matches!(img.code_type, 0x00 | 0x03) {
                    continue;
                }
                let off = img.pcir_offset + 6;
                if push_diff(&r, report, off, &device.to_le_bytes(), "device-id (PCIR)")? {
                    changed = true;
                    if img.code_type == 0x03 {
                        report.warnings.push(format!(
                            "device-id write at 0x{off:X} targets the EFI/GOP image, outside \
                             the legacy checksum region (no checksum covers it)"
                        ));
                    }
                }
            }
            if !images.iter().any(|img| img.code_type == 0x03) {
                report.warnings.push(
                    "no EFI PCI image found - device-id cloned only into the legacy image"
                        .to_string(),
                );
            }
            changed |= push_diff(
                &r,
                report,
                map.atom_ptr + 0x18,
                &subsystem_vendor.to_le_bytes(),
                "subsystem vendor-id (ATOM header)",
            )?;
            changed |= push_diff(
                &r,
                report,
                map.atom_ptr + 0x1A,
                &subsystem_device.to_le_bytes(),
                "subsystem device-id (ATOM header)",
            )?;
            Ok(changed)
        }
        PatchOp::ImportVram { donor } => {
            let dest_vram = vram_off()?;
            let donor_map = map_rom(donor)?;
            let donor_vram = donor_map
                .vram_off
                .ok_or_else(|| anyhow::anyhow!("reference ROM has no VRAM_Info table"))?;
            let donor_r = Reader::new(donor);
            let donor_mc =
                header::master_table_offset(&donor_r, donor_map.mdt, "MC_InitParameter").ok();
            let dest_mc = header::master_table_offset(&r, map.mdt, "MC_InitParameter").ok();
            let donor_info = vram::parse_vram_info(&donor_r, donor_vram, donor_mc.unwrap_or(0))?;
            let dest_info = vram::parse_vram_info(&r, dest_vram, dest_mc.unwrap_or(0))?;
            if (donor_info.fmt_rev, donor_info.cont_rev) != (dest_info.fmt_rev, dest_info.cont_rev)
            {
                bail!(
                    "VRAM_Info format mismatch: reference is {}.{} but this ROM is {}.{} \
                     (only same-format transplants are supported)",
                    donor_info.fmt_rev,
                    donor_info.cont_rev,
                    dest_info.fmt_rev,
                    dest_info.cont_rev
                );
            }
            // The donor must be internally coherent: straps calibrate a
            // specific module, and straps are density-specific, so the
            // populated modules must agree on density and size.
            for strap in &donor_info.straps {
                let module = donor_info
                    .modules
                    .get(strap.mem_block_id as usize)
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "reference ROM is incoherent: strap block {} ({} MHz) targets a \
                             missing VRAM module ({} declared)",
                            strap.mem_block_id,
                            strap.clock_mhz,
                            donor_info.modules.len()
                        )
                    })?;
                if module.vendor_id_raw == 0 && module.part_number.is_empty() {
                    bail!(
                        "reference ROM is incoherent: strap block {} targets the empty VRAM \
                         module slot {} (no memory behind it)",
                        strap.mem_block_id,
                        strap.mem_block_id
                    );
                }
            }
            // The populated modules must agree on density and size -
            // straps are calibrated per density, so a mixed set would
            // claim one calibration for two different memories.
            validate_donor_modules(&donor_r, donor_vram, &donor_info.modules)?;
            let donor_size = donor_info.struct_size as usize;
            let donor_bytes = donor
                .get(donor_vram..donor_vram + donor_size)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "reference VRAM_Info table ({donor_size} bytes) runs past the end of the \
                     reference ROM"
                    )
                })?;
            // Room check: the transplanted table must fit up to the next
            // table referenced by the destination's Master Data Table.
            let next_table = (0..header::MASTER_TABLE_NAMES.len())
                .filter_map(|i| r.u16(map.mdt + 4 + i * 2).ok())
                .map(usize::from)
                .filter(|&off| off > dest_vram)
                .min()
                .unwrap_or(r.len());
            if dest_vram + donor_size > next_table {
                bail!(
                    "reference VRAM_Info table ({donor_size} bytes) does not fit at \
                     0x{dest_vram:X}: the next table starts at 0x{next_table:X} (room for {} \
                     bytes) - refusing",
                    next_table - dest_vram
                );
            }
            let mut changed = push_diff(
                &r,
                report,
                dest_vram,
                donor_bytes,
                "VRAM_Info import (modules, straps, MC tuning)",
            )?;
            // Zero the residue of the old, larger table: unreferenced
            // now, but leaving stale geometry bytes behind is confusing.
            let old_end = (dest_vram + dest_info.struct_size as usize).min(next_table);
            let new_end = dest_vram + donor_size;
            if new_end < old_end {
                changed |= push_diff(
                    &r,
                    report,
                    new_end,
                    &vec![0; old_end - new_end],
                    "VRAM_Info import (zero-fill residue)",
                )?;
            }
            Ok(changed)
        }
        PatchOp::VramSizeMb {
            size_mb,
            understand,
        } => {
            if *size_mb > u16::MAX as u32 {
                bail!(
                    "--vram-size-mb {size_mb} exceeds the 65535 MB limit of the usMemorySize \
                     field"
                );
            }
            let density = if *size_mb % 1024 == 0 {
                match *size_mb / 1024 {
                    2 => 0x43,
                    4 => 0x53,
                    8 => 0x63,
                    16 => 0x73,
                    _ => bail!(
                        "no density code maps to {size_mb} MB per module (supported: \
                         2048/4096/8192/16384)"
                    ),
                }
            } else {
                bail!(
                    "--vram-size-mb {size_mb} is not a multiple of 1024 (supported: \
                     2048/4096/8192/16384)"
                )
            };
            let dest_vram = vram_off()?;
            let dest_mc = header::master_table_offset(&r, map.mdt, "MC_InitParameter").ok();
            let info = vram::parse_vram_info(&r, dest_vram, dest_mc.unwrap_or(0))?;
            // A strap block is calibrated for the requested size when it
            // belongs to a module that declares exactly that size.
            let calibrated = info.modules.iter().enumerate().any(|(i, module)| {
                module.memory_size_mb as u32 == *size_mb
                    && info
                        .straps
                        .iter()
                        .any(|strap| strap.mem_block_id as usize == i)
            });
            if !calibrated && !understand {
                let modules = info
                    .modules
                    .iter()
                    .map(|m| format!("{}: {} {} MB", m.index, m.part_number, m.memory_size_mb))
                    .collect::<Vec<_>>()
                    .join("; ");
                let straps = info
                    .straps
                    .iter()
                    .map(|s| format!("blk {} @ {} MHz", s.mem_block_id, s.clock_mhz))
                    .collect::<Vec<_>>()
                    .join(", ");
                // One concrete divergence example: the same clock being
                // calibrated differently for different blocks proves the
                // values are density-specific.
                let divergence = info
                    .straps
                    .iter()
                    .enumerate()
                    .find_map(|(i, a)| {
                        info.straps.iter().skip(i + 1).find_map(|b| {
                            if a.clock_mhz != b.clock_mhz || a.values == b.values {
                                return None;
                            }
                            let (reg, va) = a
                                .values
                                .iter()
                                .enumerate()
                                .find(|(j, v)| **v != b.values[*j])?;
                            Some(format!(
                                "; e.g. at {} MHz block {} writes 0x{va:08X} where block {} \
                                 writes 0x{:08X} (reg index {reg})",
                                a.clock_mhz, a.mem_block_id, b.mem_block_id, b.values[reg]
                            ))
                        })
                    })
                    .unwrap_or_default();
                bail!(
                    "no strap block is calibrated for {size_mb} MB: this ROM declares \
                     {modules} with straps {straps} (values are density-specific{divergence}). \
                     Refusing to write. Use --i-understand-strap-mismatch to force a \
                     geometry-only edit (usMemorySize/ucDensity change; timing stays unchanged)"
                );
            }
            let mut changed = false;
            let mut p = dest_vram + 20;
            for i in 0..info.num_modules as usize {
                changed |= push_diff(
                    &r,
                    report,
                    p + 14,
                    &[density],
                    &format!("VRAM module {i} density (ucDensity) -> 0x{density:02X}"),
                )?;
                changed |= push_diff(
                    &r,
                    report,
                    p + 20,
                    &(*size_mb as u16).to_le_bytes(),
                    &format!("VRAM module {i} size (usMemorySize) -> {size_mb} MB"),
                )?;
                let advance = r.u16(p + 4)? as usize;
                p += if advance > 0 { advance } else { 64 };
            }
            if !changed {
                bail!("--vram-size-mb {size_mb} is already declared (geometry unchanged)");
            }
            if !calibrated {
                report.warnings.push(format!(
                    "--vram-size-mb {size_mb} writes geometry only: the straps remain \
                     calibrated for another density (timing unchanged)"
                ));
            }
            Ok(true)
        }
    }
}

/// Checks that every populated VRAM module of the reference declares the
/// same (density, size): straps are calibrated per density, so a donor
/// mixing them would import a set that only fits part of its own
/// modules. "Populated" means the module carries a vendor id or a part
/// number - the empty placeholder slots some ROMs ship are ignored.
fn validate_donor_modules(
    r: &Reader,
    vram_off: usize,
    modules: &[crate::rom::types::VramModule],
) -> Result<()> {
    let mut populated = Vec::new();
    let mut p = vram_off + 20;
    for module in modules {
        if module.vendor_id_raw != 0 || !module.part_number.is_empty() {
            populated.push((r.u8(p + 14)?, module.memory_size_mb));
        }
        let advance = r.u16(p + 4)? as usize;
        p += if advance > 0 { advance } else { 64 };
    }
    if populated.is_empty() {
        bail!("reference ROM declares no populated VRAM module - nothing to import");
    }
    let (density, size) = populated[0];
    for (d, s) in &populated[1..] {
        if *d != density || *s != size {
            bail!(
                "reference ROM is incoherent: VRAM modules mix densities (0x{density:02X}/\
                 {size} MB vs 0x{d:02X}/{s} MB) - straps are calibrated per density, refusing \
                 to import a mixed set"
            );
        }
    }
    Ok(())
}
