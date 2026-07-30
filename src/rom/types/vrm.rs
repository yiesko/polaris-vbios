use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct EvvEntry {
    pub dpm_sclk_mhz: f64,
    pub v_adj_offset_mv: i32,
    pub dpm_v_index: u8,
    pub dpm_state: u8,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind")]
pub enum VoltageObjectDetail {
    GpioLut {
        gpio_cntl_id: u8,
        phase_delay_us: u8,
        gpio_mask: u32,
        lut_mv: Vec<u16>,
    },
    I2cInitSeq {
        regulator_id: u8,
        regulator_name: Option<String>,
        i2c_line: u8,
        i2c_address: u8,
        /// Raw (code, value) pairs from the initialization sequence.
        /// The official header names the fields "voltage code"/"voltage
        /// value (mV)", but real data shows this is not always a
        /// voltage - in several cases it is a register write to the
        /// regulator (address + raw value). So they remain as raw
        /// pairs here, without assuming a unit.
        init_pairs: Vec<(u16, u16)>,
    },
    Svid2 {
        svd_gpio_id: u8,
        svc_gpio_id: u8,
        load_line_psi_raw: u16,
    },
    Evv {
        entries: Vec<EvvEntry>,
    },
    LeakageLut {
        entries_count: u8,
    },
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct VoltageObject {
    pub voltage_type_raw: u8,
    pub voltage_type_name: String,
    pub mode_raw: u8,
    pub mode_name: String,
    pub size: u16,
    pub detail: VoltageObjectDetail,
}

#[derive(Debug, Clone, Serialize)]
pub struct VrmInfo {
    pub fmt_rev: u8,
    pub cont_rev: u8,
    pub recognized_format: bool,
    pub objects: Vec<VoltageObject>,
}
