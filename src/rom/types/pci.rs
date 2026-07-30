use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PciImage {
    pub index: usize,
    pub file_offset: usize,
    pub pcir_offset: usize,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class_code: u32,
    pub class_name: Option<String>,
    pub declared_size_bytes: usize,
    pub revision_level: u16,
    pub code_type: u8,
    pub code_type_name: String,
    pub is_last_image: bool,
    pub is_atom_bios: bool,
    pub identity_string: Option<String>,
    pub pcir_struct_length: u16,
    pub pcir_struct_revision: u8,
}
