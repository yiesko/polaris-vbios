use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct VramModule {
    pub index: usize,
    pub part_number: String,
    pub memory_size_mb: u16,
    pub memory_type_raw: u8,
    pub memory_type_name: String,
    pub channel_num: u8,
    pub vendor_id_raw: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryStrap {
    pub clock_mhz: f64,
    pub effective_gbps: f64,
    pub mem_block_id: u8,
    pub values: Vec<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VramInfo {
    pub struct_size: u16,
    pub fmt_rev: u8,
    pub cont_rev: u8,
    pub num_modules: u8,
    pub modules: Vec<VramModule>,
    pub strap_reg_indices: Vec<u16>,
    pub straps: Vec<MemoryStrap>,
    /// MC ucode (microcode) version from `ATOM_MC_INIT_PARAM_TABLE_V2_1`.
    pub mcu_code_version: Option<u32>,
    pub mcu_code_rom_start_addr: Option<u32>,
    pub mcu_code_length: Option<u32>,
}
