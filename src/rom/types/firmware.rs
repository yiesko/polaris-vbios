use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct FirmwareInfo {
    pub struct_size: u16,
    pub fmt_rev: u8,
    pub cont_rev: u8,
    pub firmware_revision: u32,
    pub default_engine_clock_mhz: f64,
    pub default_memory_clock_mhz: f64,
    pub core_ref_clock_mhz: f64,
    pub mem_ref_clock_mhz: f64,
    pub bootup_vddc_mv: u16,
    pub bootup_vddci_mv: u16,
    pub bootup_mvddc_mv: u16,
    pub bootup_vddgfx_mv: u16,
    pub memory_module_id: u8,
    pub spll_output_mhz: f64,
    pub gpull_output_mhz: f64,
    pub max_pixel_clock_pll_mhz: f64,
    pub default_disp_engine_clk_mhz: f64,
    pub min_pixel_clock_pll_input_mhz: f64,
    pub max_pixel_clock_pll_input_mhz: f64,
    pub min_pixel_clock_pll_output_mhz: f64,
    pub uniphy_dp_mode_ext_clk_mhz: f64,
    pub cooling_solution_id: u8,
    pub cooling_solution_name: String,
    pub branding_id: u8,
    pub embedded_cap: u8,
    /// VRAM regions the BIOS/driver reserve at boot (VRAM_UsageByFirmware).
    pub vram_reserves: Vec<FirmwareVramReserve>,
}

/// One VRAM region reserved by the firmware/driver, from
/// `ATOM_FIRMWARE_VRAM_RESERVE_INFO_V1_5` in the VRAM_UsageByFirmware
/// table (start address + size in KiB).
#[derive(Debug, Clone, Serialize)]
pub struct FirmwareVramReserve {
    pub start_addr: u32,
    /// KiB reserved for the BIOS firmware itself.
    pub firmware_use_kb: u16,
    /// KiB reserved for the driver framebuffer.
    pub driver_use_kb: u16,
}
