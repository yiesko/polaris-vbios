use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ThermalController {
    pub rev: u8,
    pub kind: u8,
    pub kind_name: String,
    pub i2c_line: u8,
    pub i2c_addr: u8,
    pub fan_params: u8,
    pub fan_min_rpm_x100: u8,
    pub fan_max_rpm_x100: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct StateEntry {
    pub engine_clock_index: u16,
    pub memory_clock_index: u16,
    pub pcie_gen: u16,
    pub pcie_lane: u16,
    pub classification: u16,
    pub classification_decoded: Vec<String>,
    pub caps: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct SclkEntry {
    pub level: usize,
    pub sclk_mhz: f64,
    pub vdd_index: u8,
    pub vddc_offset_mv: i32,
    pub edc_current: u16,
    pub reliability_temp_c: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct MclkEntry {
    pub level: usize,
    pub mclk_mhz: f64,
    pub vddc_index: u8,
    pub vddc_resolved_mv: Option<u16>,
    pub vddci_mv: u16,
    pub mvdd_mv: u16,
    pub vddgfx_offset_mv: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct VoltageLutEntry {
    pub index: usize,
    pub vdd_mv: u16,
    pub valid: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MmEntry {
    pub vddc_index: u8,
    pub uvd_dclk_mhz: f64,
    pub uvd_vclk_mhz: f64,
    pub vce_eclk_mhz: f64,
    pub samu_clk_mhz: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PowerTune {
    pub revid: u8,
    pub tdp_w: u16,
    pub configurable_tdp_w: u16,
    pub tdc_a: u16,
    pub battery_power_limit_w: u16,
    pub small_power_limit_w: u16,
    pub max_power_delivery_limit_w: u16,
    pub tjmax_c: u16,
    pub software_shutdown_temp_c: u16,
    pub temp_limit_hotspot_c: u16,
    pub temp_limit_liquid1_c: u16,
    pub temp_limit_liquid2_c: u16,
    pub temp_limit_vr_vddc_c: u16,
    pub temp_limit_vr_mvdd_c: u16,
    pub temp_limit_plx_c: u16,
    pub boost_power_limit_w: u16,
}

#[derive(Debug, Clone, Serialize)]
pub struct FanTable {
    pub rev: u8,
    pub t_hyst_c: u8,
    pub t_min_c: f64,
    pub t_med_c: f64,
    pub t_high_c: f64,
    pub t_max_c: f64,
    pub pwm_min_pct: f64,
    pub pwm_med_pct: f64,
    pub pwm_high_pct: f64,
    pub fan_control_mode: u8,
    pub fan_pwm_max_pct: u16,
    pub fan_rpm_max: u16,
    pub target_temperature_c: u8,
    pub minimum_pwm_limit_pct: u8,
    pub zero_rpm_enable: u8,
    pub fan_stop_temperature_c: u8,
    pub fan_start_temperature_c: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct PcieEntry {
    pub pcie_gen: u8,
    pub pcie_lane_width: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct PowerPlay {
    pub header_fmt_rev: u8,
    pub header_cont_rev: u8,
    pub table_revision: u8,
    pub struct_size_total: u16,
    pub platform_caps: u32,
    pub platform_caps_decoded: Vec<String>,
    pub max_overdrive_engine_mhz: f64,
    pub max_overdrive_memory_mhz: f64,
    pub power_control_limit_pct: u16,
    pub states: Vec<StateEntry>,
    pub thermal_controller: Option<ThermalController>,
    pub sclk_table: Vec<SclkEntry>,
    pub mclk_table: Vec<MclkEntry>,
    pub vddc_lut: Vec<VoltageLutEntry>,
    pub vddgfx_lut: Vec<VoltageLutEntry>,
    pub mm_table: Vec<MmEntry>,
    pub powertune: Option<PowerTune>,
    pub fan_table: Option<FanTable>,
    pub pcie_table: Vec<PcieEntry>,
    pub vrhot_sclk_dpm_index: Option<u8>,
    pub vce_states: Vec<VceStateEntry>,
    pub hard_limits: Vec<HardLimitEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VceStateEntry {
    pub index: usize,
    pub vce_clock_index: u8,
    pub flag: u8,
    pub sclk_index: u8,
    pub mclk_index: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct HardLimitEntry {
    pub sclk_limit_mhz: f64,
    pub mclk_limit_mhz: f64,
    pub vddc_limit_mv: u16,
    pub vddci_limit_mv: u16,
    pub vddgfx_limit_mv: u16,
}
