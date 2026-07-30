use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct EncoderRef {
    pub raw: u16,
    pub object_type_raw: u8,
    pub object_type_name: String,
    pub enum_instance: u16,
    pub chip_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DisplayPath {
    pub device_tag_raw: u16,
    pub device_tag_name: String,
    pub connector: EncoderRef,
    pub encoder_chain: Vec<EncoderRef>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DisplayInfo {
    pub recognized_format: bool,
    pub paths: Vec<DisplayPath>,
    pub supported_devices_bitmap: Option<u16>,
    pub supported_devices_names: Vec<String>,
}
