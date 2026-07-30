use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct RomHeader {
    pub file_size: usize,
    pub atom_header_offset: usize,
    pub atom_struct_size: u16,
    pub atom_fmt_rev: u8,
    pub atom_cont_rev: u8,
    pub master_data_table_offset: usize,
    pub master_cmd_table_offset: usize,
    pub subsystem_vendor_id: u16,
    pub subsystem_device_id: u16,
    pub subsystem_vendor_name: Option<String>,
    pub checksum_declared_size_blocks: u8,
    pub checksum_checked_bytes: usize,
    pub checksum_computed_sum: u8,
    pub checksum_valid: bool,
    pub bios_bootup_message: Option<String>,
    /// Internal config file name of the BIOS build (usConfigFilenameOffset).
    pub config_filename: Option<String>,
    pub build_date_candidates: Vec<String>,
    /// Names of the command tables present (offset != 0) in the
    /// `ATOM_MASTER_LIST_OF_COMMAND_TABLES`.
    pub command_tables_present: Vec<String>,
}
