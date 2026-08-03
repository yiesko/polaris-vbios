use anyhow::{Context, Result, bail};

use super::limits::HardLimitRec;
use super::limits::find_strap;
use super::map::{RomMap, map_rom, overlaps_layout, structure_contains};
use super::types::{Diff, PatchOp, PatchReport};
use crate::rom::locate;
use crate::rom::reader::Reader;

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
                &format!("PowerTune TDP: {old} W -> {watts} W"),
            )
        }
        PatchOp::Hex { offset, bytes } => {
            guard_hex(map, &r, report, *offset, bytes.len())?;
            push_diff(&r, report, *offset, bytes, "raw hex")
        }
    }
}
