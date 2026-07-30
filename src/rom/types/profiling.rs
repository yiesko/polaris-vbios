use serde::Serialize;

/// `EFUSE_LINEAR_FUNC_PARAM` - efuse address/bit description for a
/// linear (a*v+b) fuse parameter.
#[derive(Debug, Clone, Serialize)]
pub struct EfuseLinearFuncParam {
    pub efuse_index: u16,
    pub efuse_bit_lsb: u8,
    pub efuse_length: u8,
    /// Max - Min; bit 31 set means the efuse value is negative.
    pub efuse_encode_range: u32,
    pub efuse_min: u32,
}

/// `ATOM_ASIC_PROFILING_INFO_V3_6` - per-die calibration parameters:
/// the VDDC range the die tolerates (max/min), where the leakage and
/// RO fuses live, EVV/AVFS coefficients and the TDC current limit per
/// DPM state. Polaris 10/11 only (Polaris 12 uses older revisions).
#[derive(Debug, Clone, Serialize)]
pub struct ProfilingInfo {
    pub struct_size: u16,
    pub fmt_rev: u8,
    pub cont_rev: u8,
    /// ulMaxVddc, in 0.01 mV (120000 = 1200 mV).
    pub max_vddc_mv: u32,
    /// ulMinVddc, in 0.01 mV (75000 = 750 mV).
    pub min_vddc_mv: u32,
    /// usLkgEuseIndex - efuse DWORD address of the leakage fuse.
    pub lkg_euse_index: u16,
    /// ucLkgEfuseBitLSB - bit offset inside that DWORD.
    pub lkg_efuse_bit_lsb: u8,
    /// ucLkgEfuseLength - number of bits of the leakage fuse.
    pub lkg_efuse_length: u8,
    /// ulLkgEncodeLn_MaxDivMin, unit 1/100000.
    pub lkg_encode_ln_max_div_min: u32,
    /// ulLkgEncodeMax, unit 1/100000.
    pub lkg_encode_max: u32,
    /// ulLkgEncodeMin, unit 1/100000.
    pub lkg_encode_min: u32,
    /// sRoFuse - RO (ring oscillator) efuse parameters.
    pub ro_fuse: EfuseLinearFuncParam,
    /// ulEvvDefaultVddc, unit 1/100000 V (115000 = 1.15 V).
    pub evv_default_vddc_v100000: u32,
    /// ulEvvNoCalcVddc, unit 1/100000 V.
    pub evv_no_calc_vddc_v100000: u32,
    /// ulLoadLineSlop - load line slope, used as /1000 by amdgpu.
    pub load_line_slop: u32,
    /// ulaTDClimitPerDPM[8] - current limit per DPM state, unit 0.1 A
    /// (650 = 65 A).
    pub tdc_limit_per_dpm_a10: Vec<u32>,
    /// ulaNoCalcVddcPerDPM[8] - VDDC to use when EVV calculation
    /// fails, unit 1/1000000 V (1150000 = 1.15 V).
    pub no_calc_vddc_per_dpm_v1000000: Vec<u32>,
    /// usMaxVoltage_0_25mv - max VDDC in 0.25 mV (4800 = 1200 mV).
    pub max_voltage_0_25mv: u16,
    /// ucEnableGB_VDROOP_TABLE_CKSOFF
    pub enable_gb_vdroop_cksoff: bool,
    /// ucEnableGB_VDROOP_TABLE_CKSON
    pub enable_gb_vdroop_ckson: bool,
    /// ucEnableGB_FUSE_TABLE_CKSOFF
    pub enable_gb_fuse_cksoff: bool,
    /// ucEnableGB_FUSE_TABLE_CKSON
    pub enable_gb_fuse_ckson: bool,
}
