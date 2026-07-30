use anyhow::Result;

use super::reader::Reader;
use super::types::{MemoryStrap, VramInfo, VramModule};

fn memory_type_name(raw: u8) -> &'static str {
    // MC_MISC0__MEMORY_TYPE__GDDR5 = 0x50 (confirmed in atombios.h)
    match raw {
        0x50 => "GDDR5",
        0x40 => "DDR4",
        0x30 => "DDR3",
        0x20 => "DDR2",
        0x10 => "DDR1",
        _ => "unknown",
    }
}

fn parse_vram_module(r: &Reader, off: usize, index: usize) -> Result<(VramModule, usize)> {
    let module_size = r.u16(off + 4)? as usize;
    let memory_type_raw = r.u8(off + 11)?;
    let channel_num = r.u8(off + 12)?;
    let memory_size_mb = r.u16(off + 20)?;
    let vendor_id_raw = r.u8(off + 28)?;
    let part_number = r.cstr(off + 44, 20)?;

    let module = VramModule {
        index,
        part_number,
        memory_size_mb,
        memory_type_raw,
        memory_type_name: memory_type_name(memory_type_raw).to_string(),
        channel_num,
        vendor_id_raw,
    };
    let advance = if module_size > 0 { module_size } else { 64 };
    Ok((module, advance))
}

/// Parses the memory clock patch table (`ATOM_INIT_REG_BLOCK`) -
/// these are the memory "straps": a set of memory controller (MC)
/// register values per clock range (1500/1625/1750/2000 MHz etc.),
/// per memory block/vendor.
fn parse_memory_straps(
    r: &Reader,
    base_off: usize,
    rel_off: usize,
) -> Result<(Vec<u16>, Vec<MemoryStrap>)> {
    if rel_off == 0 {
        return Ok((Vec::new(), Vec::new()));
    }
    let off = base_off + rel_off;
    let reg_index_tbl_size = r.u16(off)? as usize;
    let reg_data_blk_size = r.u16(off + 2)? as usize;
    let n_regs = reg_index_tbl_size / 3;

    let mut reg_indices = Vec::with_capacity(n_regs);
    let mut p = off + 4;
    for _ in 0..n_regs {
        reg_indices.push(r.u16(p)?);
        p += 3;
    }

    let n_vals = reg_data_blk_size / 4;
    let n_vals = n_vals.saturating_sub(1);
    let mut data_start = p;
    let mut straps = Vec::new();
    let mut idx = 0usize;
    while data_start + reg_data_blk_size <= r.len() && idx < 64 {
        let mem_id_raw = r.u32(data_start)?;
        let clock_range = mem_id_raw & 0x00FF_FFFF;
        let blk_id = ((mem_id_raw >> 24) & 0xFF) as u8;
        if clock_range == 0 && blk_id == 0 && idx > 0 {
            break;
        }
        let mut values = Vec::with_capacity(n_vals);
        let mut vp = data_start + 4;
        for _ in 0..n_vals {
            values.push(r.u32(vp)?);
            vp += 4;
        }
        let clock_mhz = clock_range as f64 / 100.0;
        straps.push(MemoryStrap {
            clock_mhz,
            effective_gbps: clock_mhz * 4.0 / 1000.0,
            mem_block_id: blk_id,
            values,
        });
        data_start += reg_data_blk_size;
        idx += 1;
    }

    Ok((reg_indices, straps))
}

/// Parses the memory controller ucode info from
/// `ATOM_MC_INIT_PARAM_TABLE_V2_1` (atombios.h line 7520). Returns
/// (ucode version, ucode ROM start address, ucode length), or None if
/// the table uses the older `V1` format (no ucode fields).
fn parse_mc_ucode(r: &Reader, off: usize) -> Option<(u32, u32, u32)> {
    if off == 0 {
        return None;
    }
    let struct_size = r.u16(off).ok()?;
    let fmt_rev = r.u8(off + 2).ok()?;
    let cont_rev = r.u8(off + 3).ok()?;
    if fmt_rev != 2 || cont_rev != 1 || struct_size < 20 {
        return None;
    }
    Some((
        r.u32(off + 4).ok()?,
        r.u32(off + 8).ok()?,
        r.u32(off + 12).ok()?,
    ))
}

/// Parses `ATOM_VRAM_INFO_HEADER_V2_2` (the format used by Polaris,
/// with `ATOM_VRAM_MODULE_V8` modules).
pub fn parse_vram_info(r: &Reader, off: usize, mc_init_off: usize) -> Result<VramInfo> {
    let struct_size = r.u16(off)?;
    let fmt_rev = r.u8(off + 2)?;
    let cont_rev = r.u8(off + 3)?;
    let mem_clk_patch_off = r.u16(off + 6)? as usize;
    let num_modules = r.u8(off + 16)?;

    let mut modules = Vec::with_capacity(num_modules as usize);
    let mut p = off + 20;
    for i in 0..num_modules as usize {
        let (module, advance) = parse_vram_module(r, p, i)?;
        modules.push(module);
        p += advance;
    }

    let (strap_reg_indices, straps) = parse_memory_straps(r, off, mem_clk_patch_off)?;

    let (mcu_code_version, mcu_code_rom_start_addr, mcu_code_length) =
        match parse_mc_ucode(r, mc_init_off) {
            Some((v, s, l)) => (Some(v), Some(s), Some(l)),
            None => (None, None, None),
        };

    Ok(VramInfo {
        struct_size,
        fmt_rev,
        cont_rev,
        num_modules,
        modules,
        strap_reg_indices,
        straps,
        mcu_code_version,
        mcu_code_rom_start_addr,
        mcu_code_length,
    })
}
