use anyhow::Result;

use super::reader::Reader;
use super::types::AsicInfo;

/// Parses `ATOM_GFX_INFO_V2_1` / `ATOM_GFX_INFO_V2_3` (GFX_Info table,
/// atombios.h line 5647). V2_3 adds two EDC/DIDT table offsets after the
/// byte counters. The table carries the physical ASIC layout (number of
/// shader engines, CUs per SH, render backends, ...) which identifies
/// the die configuration of the GPU.
pub fn parse_asic_info(r: &Reader, off: usize) -> Result<AsicInfo> {
    let struct_size = r.u16(off)?;
    let fmt_rev = r.u8(off + 2)?;
    let cont_rev = r.u8(off + 3)?;
    let is_v23 = struct_size >= 20;

    Ok(AsicInfo {
        struct_size,
        fmt_rev,
        cont_rev,
        gfx_ip_min_ver: r.u8(off + 4)?,
        gfx_ip_maj_ver: r.u8(off + 5)?,
        max_shader_engines: r.u8(off + 6)?,
        max_tile_pipes: r.u8(off + 7)?,
        max_cu_per_sh: r.u8(off + 8)?,
        max_sh_per_se: r.u8(off + 9)?,
        max_backends_per_se: r.u8(off + 10)?,
        max_texture_channel_caches: r.u8(off + 11)?,
        hi_lo_leakage_threshold: if is_v23 { Some(r.u16(off + 12)?) } else { None },
        edc_didt_lo_dpm7_offset: if is_v23 { Some(r.u16(off + 14)?) } else { None },
        edc_didt_hi_dpm7_offset: if is_v23 { Some(r.u16(off + 16)?) } else { None },
    })
}
