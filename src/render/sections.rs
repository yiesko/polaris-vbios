use std::fmt;

/// Each category of information that can be displayed, alone or combined,
/// both via `--sections` on the CLI and browsable in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Section {
    Header,
    PcirChain,
    Display,
    Firmware,
    Sclk,
    Mclk,
    Voltages,
    Vrm,
    Multimedia,
    Powertune,
    Fan,
    Pcie,
    Vram,
    Straps,
    Caps,
    Asic,
    Smu,
    Power,
    Gpio,
    Profiling,
    Ss,
    Vesa,
    I2c,
}

impl Section {
    pub const ALL: [Section; 23] = [
        Section::Header,
        Section::PcirChain,
        Section::Display,
        Section::Firmware,
        Section::Sclk,
        Section::Mclk,
        Section::Voltages,
        Section::Vrm,
        Section::Multimedia,
        Section::Powertune,
        Section::Fan,
        Section::Pcie,
        Section::Vram,
        Section::Straps,
        Section::Caps,
        Section::Asic,
        Section::Smu,
        Section::Power,
        Section::Gpio,
        Section::Profiling,
        Section::Ss,
        Section::Vesa,
        Section::I2c,
    ];

    pub fn key(&self) -> &'static str {
        match self {
            Section::Header => "header",
            Section::PcirChain => "pcir",
            Section::Display => "display",
            Section::Firmware => "firmware",
            Section::Sclk => "sclk",
            Section::Mclk => "mclk",
            Section::Voltages => "voltages",
            Section::Vrm => "vrm",
            Section::Multimedia => "mm",
            Section::Powertune => "powertune",
            Section::Fan => "fan",
            Section::Pcie => "pcie",
            Section::Vram => "vram",
            Section::Straps => "straps",
            Section::Caps => "caps",
            Section::Asic => "asic",
            Section::Smu => "smu",
            Section::Power => "power",
            Section::Gpio => "gpio",
            Section::Profiling => "profiling",
            Section::Ss => "ss",
            Section::Vesa => "vesa",
            Section::I2c => "i2c",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Section::Header => "Identification (ROM/ATOM/PCB)",
            Section::PcirChain => "PCI Option ROM chain (chip images)",
            Section::Display => "Video outputs (connectors/encoders)",
            Section::Firmware => "Firmware / boot clocks",
            Section::Sclk => "P-States - SCLK (engine clock)",
            Section::Mclk => "P-States - MCLK (memory)",
            Section::Voltages => "Voltage tables (VDDC/VDDGFX)",
            Section::Vrm => "VRM (voltage control / VoltageObjectInfo)",
            Section::Multimedia => "Multimedia clocks (UVD/VCE/SAMU)",
            Section::Powertune => "PowerTune (TDP / TDC / thermal limits)",
            Section::Fan => "Fan curve",
            Section::Pcie => "PCIe table",
            Section::Vram => "VRAM configuration",
            Section::Straps => "Memory straps",
            Section::Caps => "Platform caps (PowerPlay table flags)",
            Section::Asic => "ASIC layout (GFX_Info)",
            Section::Smu => "SMU (firmware version / FCW ranges)",
            Section::Power => "Power sources (connectors / sensors)",
            Section::Gpio => "GPIO pin roles (GPIO_Pin_LUT)",
            Section::Profiling => "ASIC profiling (die voltage range / efuse / TDC)",
            Section::Ss => "Spread spectrum (ASIC_InternalSS_Info)",
            Section::Vesa => "Native VESA modes (StandardVESA_Timing)",
            Section::I2c => "I2C bus wiring (GPIO_I2C_Info)",
        }
    }

    pub fn from_key(s: &str) -> Option<Section> {
        Section::ALL.into_iter().find(|sec| sec.key() == s.trim())
    }
}

impl fmt::Display for Section {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Parses a comma-separated list (e.g. "firmware,straps,vram").
/// The special word "all" selects everything.
pub fn parse_section_list(spec: &str) -> Result<Vec<Section>, String> {
    if spec.trim().eq_ignore_ascii_case("all") {
        return Ok(Section::ALL.to_vec());
    }
    let mut out = Vec::new();
    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        match Section::from_key(part) {
            Some(sec) => out.push(sec),
            None => {
                return Err(format!(
                    "unknown section: '{part}'. Use --list-sections to see available options."
                ));
            }
        }
    }
    Ok(out)
}
