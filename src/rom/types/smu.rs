use serde::Serialize;

/// `ATOM_SCLK_FCW_RANGE_ENTRY_V1` inside SMU_Info.
#[derive(Debug, Clone, Serialize)]
pub struct SclkFcwRangeEntry {
    pub index: usize,
    pub max_sclk_mhz: f64,
    /// 1 = 3-6 GHz VCO, 3 = 2-4 GHz VCO.
    pub vco_setting: u8,
    pub vco_setting_name: String,
    /// Post divider (divide by 2^n).
    pub postdiv: u8,
    pub fcw_pcc: u16,
    pub fcw_trans_upper: u16,
    pub rcw_trans_lower: u16,
}

/// `ATOM_SMU_INFO_V2_1` - SMU version + SCLK FCW ranges (Polaris SMU7).
#[derive(Debug, Clone, Serialize)]
pub struct SmuInfo {
    pub struct_size: u16,
    pub fmt_rev: u8,
    pub cont_rev: u8,
    pub sclk_entry_num: u8,
    pub smu_ver: u8,
    pub share_power_source: u8,
    pub fcw_entries: Vec<SclkFcwRangeEntry>,
}
