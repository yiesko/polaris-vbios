use serde::Serialize;

/// `ATOM_GFX_INFO_V2_1` / `V2_3` - ASIC layout (GFX IP version,
/// compute unit / shader engine / render backend counts).
#[derive(Debug, Clone, Serialize)]
pub struct AsicInfo {
    pub struct_size: u16,
    pub fmt_rev: u8,
    pub cont_rev: u8,
    pub gfx_ip_min_ver: u8,
    pub gfx_ip_maj_ver: u8,
    pub max_shader_engines: u8,
    pub max_tile_pipes: u8,
    pub max_cu_per_sh: u8,
    pub max_sh_per_se: u8,
    pub max_backends_per_se: u8,
    pub max_texture_channel_caches: u8,
    /// V2_3 only.
    pub hi_lo_leakage_threshold: Option<u16>,
    /// V2_3 only: offsets of the EDC/DIDT DPM7 low/high leakage tables.
    pub edc_didt_lo_dpm7_offset: Option<u16>,
    pub edc_didt_hi_dpm7_offset: Option<u16>,
}
