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
