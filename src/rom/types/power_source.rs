use serde::Serialize;

/// `ATOM_POWER_SOURCE_OBJECT` - one power input (PCIe slot, 6-pin, 8-pin)
/// and how its presence is sensed.
#[derive(Debug, Clone, Serialize)]
pub struct PowerSourceObject {
    pub index: usize,
    pub source_id_raw: u8,
    pub source_name: String,
    pub sensor_type_raw: u8,
    pub sensor_type_name: String,
    pub sensor_id: u8,
    pub sensor_slave_addr: u8,
    pub sensor_reg_index: u8,
    pub sensor_reg_bit_mask: u8,
    pub sensor_active_state: u8,
    /// Sensed power in watts (0 if unknown).
    pub sensed_power_w: u16,
}

/// `ATOM_POWER_SOURCE_INFO` - list of power sources the VBIOS knows
/// how to detect.
#[derive(Debug, Clone, Serialize)]
pub struct PowerSourceInfo {
    pub struct_size: u16,
    pub fmt_rev: u8,
    pub cont_rev: u8,
    pub objects: Vec<PowerSourceObject>,
}
