use anyhow::Result;

use super::reader::Reader;
use super::types::{SclkFcwRangeEntry, SmuInfo};

/// VCO setting in `ATOM_SCLK_FCW_RANGE_ENTRY_V1` (atombios.h line 5630):
/// 1 = 3-6 GHz VCO, 3 = 2-4 GHz VCO.
fn vco_setting_name(v: u8) -> &'static str {
    match v {
        1 => "3-6 GHz VCO",
        3 => "2-4 GHz VCO",
        _ => "unknown",
    }
}

/// Parses `ATOM_SMU_INFO_V2_1` (SMU_Info table, atombios.h line 5635) -
/// the SMU7 firmware version and the SCLK FCW (fractional clock word)
/// ranges used by the VCO/post-divider configuration.
pub fn parse_smu_info(r: &Reader, off: usize) -> Result<SmuInfo> {
    let (struct_size, fmt_rev, cont_rev) = r.table_header(off)?;

    let sclk_entry_num = r.u8(off + 4)?;
    let smu_ver = r.u8(off + 5)?;
    let share_power_source = r.u8(off + 6)?;

    // ATOM_SCLK_FCW_RANGE_ENTRY_V1 is 12 bytes, up to 8 entries.
    let entry_base = off + 8;
    let max_entries = sclk_entry_num.min(8);
    let avail = struct_size.saturating_sub(8) / 12;
    let n = max_entries.min(avail as u8) as usize;

    let mut fcw_entries = Vec::with_capacity(n);
    for i in 0..n {
        let p = entry_base + i * 12;
        let vco = r.u8(p + 4)?;
        fcw_entries.push(SclkFcwRangeEntry {
            index: i,
            max_sclk_mhz: r.u32(p)? as f64 / 100.0,
            vco_setting: vco,
            vco_setting_name: vco_setting_name(vco).to_string(),
            postdiv: r.u8(p + 5)?,
            fcw_pcc: r.u16(p + 6)?,
            fcw_trans_upper: r.u16(p + 8)?,
            rcw_trans_lower: r.u16(p + 10)?,
        });
    }

    Ok(SmuInfo {
        struct_size,
        fmt_rev,
        cont_rev,
        sclk_entry_num,
        smu_ver,
        share_power_source,
        fcw_entries,
    })
}
