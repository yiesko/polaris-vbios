use serde::Serialize;

use super::{
    asic::AsicInfo, display::DisplayInfo, firmware::FirmwareInfo, gpio::GpioPinLut,
    gpio_i2c::GpioI2cInfo, header::RomHeader, pci::PciImage, power::PowerPlay,
    power_source::PowerSourceInfo, profiling::ProfilingInfo, smu::SmuInfo, ss::SsInfo,
    vesa::VesaInfo, vram::VramInfo, vrm::VrmInfo,
};

#[derive(Debug, Clone, Serialize)]
pub struct ParsedRom {
    pub file_name: String,
    pub header: RomHeader,
    pub firmware: FirmwareInfo,
    pub powerplay: PowerPlay,
    pub vram: VramInfo,
    pub vrm: VrmInfo,
    pub pci_images: Vec<PciImage>,
    pub display: DisplayInfo,
    pub asic: Option<AsicInfo>,
    pub smu: Option<SmuInfo>,
    pub power_source: Option<PowerSourceInfo>,
    pub gpio_pin_lut: Option<GpioPinLut>,
    pub profiling: Option<ProfilingInfo>,
    pub ss: Option<SsInfo>,
    pub vesa: Option<VesaInfo>,
    pub i2c: Option<GpioI2cInfo>,
    pub warnings: Vec<String>,
}
