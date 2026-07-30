use super::reader::Reader;

/// Absolute offset of a sub-table inside the PowerPlay table, given its
/// relative offset field inside `ATOM_Tonga_POWERPLAYTABLE` (`off + rel`).
/// Returns `None` when the ROM does not include the sub-table (field 0).
fn pp_subtable(r: &Reader, pp_off: usize, rel: usize) -> Option<usize> {
    let rel_off = r.u16(pp_off + rel).ok()? as usize;
    if rel_off == 0 {
        return None;
    }
    Some(pp_off + rel_off)
}

/// PowerPlay fields (same offsets as `powerplay.rs::parse_powerplay`).
const PP_MCLK_DEP: usize = 43;
const PP_SCLK_DEP: usize = 45;
const PP_VDDC_LUT: usize = 47;
const PP_POWERTUNE: usize = 57;
const PP_HARD_LIMIT: usize = 59;

pub fn powerplay_sclk_table(r: &Reader, pp_off: usize) -> Option<usize> {
    pp_subtable(r, pp_off, PP_SCLK_DEP)
}

pub fn powerplay_mclk_table(r: &Reader, pp_off: usize) -> Option<usize> {
    pp_subtable(r, pp_off, PP_MCLK_DEP)
}

pub fn powerplay_vddc_lut(r: &Reader, pp_off: usize) -> Option<usize> {
    pp_subtable(r, pp_off, PP_VDDC_LUT)
}

pub fn powerplay_powertune(r: &Reader, pp_off: usize) -> Option<usize> {
    pp_subtable(r, pp_off, PP_POWERTUNE)
}

pub fn powerplay_hard_limits(r: &Reader, pp_off: usize) -> Option<usize> {
    pp_subtable(r, pp_off, PP_HARD_LIMIT)
}

/// Absolute offset of the SCLK DPM entry `level` (header: u8 rev + u8 n;
/// Polaris rev >= 1 records are 15 bytes, older 11 - same as
/// `powerplay.rs::parse_sclk_table`).
pub fn sclk_entry(r: &Reader, sclk_off: usize, level: usize) -> Option<usize> {
    let rev = r.u8(sclk_off).ok()?;
    let n = r.u8(sclk_off + 1).ok()? as usize;
    let entry_size = if rev >= 1 { 15 } else { 11 };
    (level < n).then(|| sclk_off + 2 + level * entry_size)
}

/// Absolute offset of the SCLK value (u32, centi-MHz) inside the SCLK
/// DPM entry `level` (`+3` inside the record).
pub fn sclk_value(r: &Reader, sclk_off: usize, level: usize) -> Option<usize> {
    sclk_entry(r, sclk_off, level).map(|e| e + 3)
}

/// Absolute offset of the MCLK DPM entry `level` (header: u8 n; records
/// are 13 bytes - same as `powerplay.rs::parse_mclk_table`).
pub fn mclk_entry(r: &Reader, mclk_off: usize, level: usize) -> Option<usize> {
    let n = r.u8(mclk_off + 1).ok()? as usize;
    (level < n).then(|| mclk_off + 2 + level * 13)
}

/// Absolute offset of the MCLK value (u32, centi-MHz) inside the MCLK
/// DPM entry `level` (`+7` inside the record).
pub fn mclk_value(r: &Reader, mclk_off: usize, level: usize) -> Option<usize> {
    mclk_entry(r, mclk_off, level).map(|e| e + 7)
}

/// Absolute offset of VDDC LUT entry `index` (header: u8 n; records are
/// 8 bytes, voltage at `+0` - same as `powerplay.rs::parse_voltage_lut`).
pub fn vddc_lut_entry(r: &Reader, lut_off: usize, index: usize) -> Option<usize> {
    let n = r.u8(lut_off + 1).ok()? as usize;
    (index < n).then(|| lut_off + 2 + index * 8)
}

/// Absolute offset of the TDP value (u16, watts) inside the PowerTune
/// table (`+1`, right after the u8 rev id - same as
/// `powerplay.rs::parse_powertune`).
pub fn powertune_tdp(pt_off: usize) -> usize {
    pt_off + 1
}

/// (data_start, block_size) of the memory strap data area - same
/// traversal as `vram.rs::parse_memory_straps`.
pub fn strap_region(r: &Reader, vram_off: usize) -> Option<(usize, usize)> {
    let patch_off = r.u16(vram_off + 6).ok()? as usize;
    if patch_off == 0 {
        return None;
    }
    let base = vram_off + patch_off;
    let reg_index_tbl_size = r.u16(base).ok()? as usize;
    let reg_data_blk_size = r.u16(base + 2).ok()? as usize;
    Some((base + 4 + reg_index_tbl_size, reg_data_blk_size))
}

/// Absolute offset of the strap clock field (u32 `mem_id_raw`, clock =
/// value & 0xFFFFFF) of strap `idx`.
pub fn strap_clock_field(r: &Reader, vram_off: usize, idx: usize) -> Option<usize> {
    let (data_start, block_size) = strap_region(r, vram_off)?;
    Some(data_start + idx * block_size)
}

/// Absolute offset of register value `reg` (u32) inside strap `idx`.
pub fn strap_value(r: &Reader, vram_off: usize, idx: usize, reg: usize) -> Option<usize> {
    strap_clock_field(r, vram_off, idx).map(|off| off + 4 + reg * 4)
}

/// Absolute offset of the MC register index `reg` (u16) inside the strap
/// register index table (index `reg` of `ATOM_MC_REG_INDEX` entries).
pub fn strap_reg_index(r: &Reader, vram_off: usize, reg: usize) -> Option<usize> {
    let patch_off = r.u16(vram_off + 6).ok()? as usize;
    if patch_off == 0 {
        return None;
    }
    let base = vram_off + patch_off;
    let reg_index_tbl_size = r.u16(base).ok()? as usize;
    let n_regs = reg_index_tbl_size / 3;
    (reg < n_regs).then(|| base + 4 + reg * 3)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Locator offsets must agree with the parsers on every sample ROM:
    /// for every strap, every SCLK/MCLK DPM level, every LUT entry and
    /// the PowerTune TDP, the value read through the locator must equal
    /// the value the parser reports.
    #[test]
    fn locators_agree_with_parsers() {
        let Some(samples) = crate::rom::test_support::sample_roms() else {
            eprintln!("samples/BIOS not present; skipping locator consistency test");
            return;
        };
        assert!(
            !samples.is_empty(),
            "no sample ROMs found under samples/BIOS"
        );
        let mut checked = 0usize;
        for path in &samples {
            let data = std::fs::read(path).expect("read sample ROM");
            let r = Reader::new(&data);
            let rom = crate::rom::parse_rom(path)
                .unwrap_or_else(|e| panic!("parse_rom failed for {}: {e:#}", path.display()));
            let atom = r.u16(0x48).unwrap() as usize;
            let mdt = r.u16(atom + 0x20).unwrap() as usize;
            let pp_off = crate::rom::header::master_table_offset(&r, mdt, "PowerPlayInfo").unwrap();
            let vram_off = crate::rom::header::master_table_offset(&r, mdt, "VRAM_Info").unwrap();

            // Straps: every register value of every strap.
            if strap_region(&r, vram_off).is_some() {
                for (si, strap) in rom.vram.straps.iter().enumerate() {
                    for (ri, value) in strap.values.iter().enumerate() {
                        let off =
                            strap_value(&r, vram_off, si, ri).expect("locator for strap value");
                        let raw = r.u32(off).unwrap();
                        assert_eq!(raw, *value, "{} strap {si} reg {ri}", path.display());
                        checked += 1;
                    }
                    let off = strap_clock_field(&r, vram_off, si).expect("locator for strap clock");
                    let raw = r.u32(off).unwrap();
                    assert_eq!(
                        (raw & 0xFF_FFFF) as f64 / 100.0,
                        strap.clock_mhz,
                        "{} strap {si} clock",
                        path.display()
                    );
                }
            }

            // SCLK DPM values.
            if let Some(sclk_off) = powerplay_sclk_table(&r, pp_off) {
                for (level, entry) in rom.powerplay.sclk_table.iter().enumerate() {
                    let off = sclk_value(&r, sclk_off, level).expect("locator for sclk");
                    let raw = r.u32(off).unwrap();
                    assert_eq!(
                        raw as f64 / 100.0,
                        entry.sclk_mhz,
                        "{} sclk level {level}",
                        path.display()
                    );
                    checked += 1;
                }
            }

            // MCLK DPM values.
            if let Some(mclk_off) = powerplay_mclk_table(&r, pp_off) {
                for (level, entry) in rom.powerplay.mclk_table.iter().enumerate() {
                    let off = mclk_value(&r, mclk_off, level).expect("locator for mclk");
                    let raw = r.u32(off).unwrap();
                    assert_eq!(
                        raw as f64 / 100.0,
                        entry.mclk_mhz,
                        "{} mclk level {level}",
                        path.display()
                    );
                    checked += 1;
                }
            }

            // VDDC LUT values.
            if let Some(lut_off) = powerplay_vddc_lut(&r, pp_off) {
                for (index, entry) in rom.powerplay.vddc_lut.iter().enumerate() {
                    let off = vddc_lut_entry(&r, lut_off, index).expect("locator for lut");
                    let raw = r.u16(off).unwrap();
                    assert_eq!(raw, entry.vdd_mv, "{} lut {index}", path.display());
                    checked += 1;
                }
            }

            // PowerTune TDP.
            if let (Some(pt_off), Some(pt)) =
                (powerplay_powertune(&r, pp_off), &rom.powerplay.powertune)
            {
                let off = powertune_tdp(pt_off);
                let raw = r.u16(off).unwrap();
                assert_eq!(raw, pt.tdp_w, "{} tdp", path.display());
                checked += 1;
            }

            // Hard limit records (14 bytes each, same as the parser).
            if let Some(hl_off) = powerplay_hard_limits(&r, pp_off) {
                let n = r.u8(hl_off + 1).unwrap() as usize;
                assert_eq!(
                    n,
                    rom.powerplay.hard_limits.len(),
                    "{} hard limits",
                    path.display()
                );
            }
        }
        eprintln!(
            "locator consistency: {checked} values checked across {} ROMs",
            samples.len()
        );
    }
}
