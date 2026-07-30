use anyhow::Result;

use super::reader::Reader;
use super::types::{EfuseLinearFuncParam, ProfilingInfo};

/// Parses `ATOM_ASIC_PROFILING_INFO_V3_6` (atombios.h line 5552),
/// the per-die calibration table of Polaris 10/11. It carries the
/// VDDC voltage envelope of the die (max/min), the addresses of the
/// leakage and RO efuses, EVV fallback voltages, the load line slope
/// and the TDC current limit per DPM state. Older content revisions
/// (V3_1..V3_5) share the first ~128 bytes; the load line / TDC /
/// no-calc-VDDC fields only exist from V3_6 on.
pub fn parse_profiling_info(r: &Reader, off: usize) -> Result<ProfilingInfo> {
    let struct_size = r.u16(off)?;
    let fmt_rev = r.u8(off + 2)?;
    let cont_rev = r.u8(off + 3)?;
    let v36 = cont_rev >= 6 && struct_size >= 132;

    let read_u32 = |at: usize| -> u32 {
        if struct_size as usize >= at + 4 {
            r.u32(off + at).unwrap_or(0)
        } else {
            0
        }
    };
    let read_u16 = |at: usize| -> u16 {
        if struct_size as usize >= at + 2 {
            r.u16(off + at).unwrap_or(0)
        } else {
            0
        }
    };

    let ro_fuse = EfuseLinearFuncParam {
        efuse_index: read_u16(28),
        efuse_bit_lsb: r.u8(off + 30).unwrap_or(0),
        efuse_length: r.u8(off + 31).unwrap_or(0),
        efuse_encode_range: read_u32(32),
        efuse_min: read_u32(36),
    };

    let load_line_slop = if v36 { read_u32(128) } else { 0 };
    let tdc_count = if v36 {
        ((struct_size.saturating_sub(132)) / 4).min(8)
    } else {
        0
    };
    let tdc_limit_per_dpm_a10: Vec<u32> = (0..tdc_count)
        .map(|i| read_u32(132 + i as usize * 4))
        .collect();

    let nocount = if v36 {
        ((struct_size.saturating_sub(164)) / 4).min(8)
    } else {
        0
    };
    let no_calc_vddc_per_dpm_v1000000: Vec<u32> = (0..nocount)
        .map(|i| read_u32(164 + i as usize * 4))
        .collect();

    Ok(ProfilingInfo {
        struct_size,
        fmt_rev,
        cont_rev,
        max_vddc_mv: read_u32(4),
        min_vddc_mv: read_u32(8),
        lkg_euse_index: read_u16(12),
        lkg_efuse_bit_lsb: r.u8(off + 14).unwrap_or(0),
        lkg_efuse_length: r.u8(off + 15).unwrap_or(0),
        lkg_encode_ln_max_div_min: read_u32(16),
        lkg_encode_max: read_u32(20),
        lkg_encode_min: read_u32(24),
        ro_fuse,
        evv_default_vddc_v100000: read_u32(40),
        evv_no_calc_vddc_v100000: read_u32(44),
        load_line_slop,
        tdc_limit_per_dpm_a10,
        no_calc_vddc_per_dpm_v1000000,
        max_voltage_0_25mv: read_u16(258),
        enable_gb_vdroop_cksoff: r.u8(off + 260).unwrap_or(0) != 0,
        enable_gb_vdroop_ckson: r.u8(off + 261).unwrap_or(0) != 0,
        enable_gb_fuse_cksoff: r.u8(off + 262).unwrap_or(0) != 0,
        enable_gb_fuse_ckson: r.u8(off + 263).unwrap_or(0) != 0,
    })
}
