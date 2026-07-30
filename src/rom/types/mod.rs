//! Types describing each parsed ATOM table, one module per table/domain.
//! Re-exported flat so callers keep using `types::ParsedRom`, etc.

mod asic;
mod display;
mod firmware;
mod gpio;
mod gpio_i2c;
mod header;
mod parsed;
mod pci;
mod power;
mod power_source;
mod profiling;
mod smu;
mod ss;
mod vesa;
mod vram;
mod vrm;

pub use asic::AsicInfo;
pub use display::{DisplayInfo, DisplayPath, EncoderRef};
pub use firmware::{FirmwareInfo, FirmwareVramReserve};
pub use gpio::{GpioPinAssignment, GpioPinLut};
pub use gpio_i2c::{GpioI2cAssignment, GpioI2cInfo};
pub use header::RomHeader;
pub use parsed::ParsedRom;
pub use pci::PciImage;
pub use power::{
    FanTable, HardLimitEntry, MclkEntry, MmEntry, PcieEntry, PowerPlay, PowerTune, SclkEntry,
    StateEntry, ThermalController, VceStateEntry, VoltageLutEntry,
};
pub use power_source::{PowerSourceInfo, PowerSourceObject};
pub use profiling::{EfuseLinearFuncParam, ProfilingInfo};
pub use smu::{SclkFcwRangeEntry, SmuInfo};
pub use ss::{SsEntry, SsInfo};
pub use vesa::{VesaInfo, VesaMode};
pub use vram::{MemoryStrap, VramInfo, VramModule};
pub use vrm::{EvvEntry, VoltageObject, VoltageObjectDetail, VrmInfo};
