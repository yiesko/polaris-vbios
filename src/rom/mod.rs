pub mod asic;
pub mod disasm;
pub mod display;
pub mod firmware;
pub mod gpio;
pub mod gpio_i2c;
pub mod header;
pub mod limits;
pub mod locate;
pub mod patch;
pub mod pci;
pub mod power_source;
pub mod powerplay;
pub mod profiling;
pub mod reader;
pub mod smu;
pub mod ss;
pub mod timings;
pub mod types;
pub mod validate;
pub mod vesa;
pub mod vram;
pub mod vrm;

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use reader::Reader;
use types::ParsedRom;

fn parse_optional<T>(
    r: &Reader<'_>,
    offset: usize,
    label: &str,
    parse_fn: fn(&Reader<'_>, usize) -> Result<T>,
) -> Result<Option<T>> {
    if offset != 0 {
        Ok(Some(parse_fn(r, offset).with_context(|| {
            format!("failed to parse {label} table")
        })?))
    } else {
        Ok(None)
    }
}

pub fn parse_rom(path: &Path) -> Result<ParsedRom> {
    let data =
        fs::read(path).with_context(|| format!("could not read file '{}'", path.display()))?;
    parse_bytes(&data, &file_name_of(path))
}

fn file_name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

/// Parses ROM bytes held in memory (used by the patch engine to
/// re-verify a patched image before it is written).
pub fn parse_bytes(data: &[u8], file_name: &str) -> Result<ParsedRom> {
    let r = Reader::new(data);

    let rom_header = header::parse_rom_header(&r)
        .with_context(|| format!("invalid ATOM header in '{file_name}'"))?;

    let fw_off =
        header::master_table_offset(&r, rom_header.master_data_table_offset, "FirmwareInfo")?;
    let pp_off =
        header::master_table_offset(&r, rom_header.master_data_table_offset, "PowerPlayInfo")?;
    let vram_off =
        header::master_table_offset(&r, rom_header.master_data_table_offset, "VRAM_Info")?;
    let vrm_off =
        header::master_table_offset(&r, rom_header.master_data_table_offset, "VoltageObjectInfo")?;
    let asic_off =
        header::master_table_offset(&r, rom_header.master_data_table_offset, "GFX_Info")?;
    let smu_off = header::master_table_offset(&r, rom_header.master_data_table_offset, "SMU_Info")?;
    let pwr_off =
        header::master_table_offset(&r, rom_header.master_data_table_offset, "PowerSourceInfo")?;
    let gpio_off =
        header::master_table_offset(&r, rom_header.master_data_table_offset, "GPIO_Pin_LUT")?;
    let mc_off =
        header::master_table_offset(&r, rom_header.master_data_table_offset, "MC_InitParameter")?;
    let prof_off = header::master_table_offset(
        &r,
        rom_header.master_data_table_offset,
        "ASIC_ProfilingInfo",
    )?;
    let ss_off = header::master_table_offset(
        &r,
        rom_header.master_data_table_offset,
        "ASIC_InternalSS_Info",
    )?;
    let vuf_off = header::master_table_offset(
        &r,
        rom_header.master_data_table_offset,
        "VRAM_UsageByFirmware",
    )?;
    let vesa_off = header::master_table_offset(
        &r,
        rom_header.master_data_table_offset,
        "StandardVESA_Timing",
    )?;
    let i2c_off =
        header::master_table_offset(&r, rom_header.master_data_table_offset, "GPIO_I2C_Info")?;

    let mut firmware =
        firmware::parse_firmware_info(&r, fw_off).context("failed to parse FirmwareInfo table")?;
    if vuf_off != 0 {
        firmware.vram_reserves = firmware::parse_vram_usage(&r, vuf_off)
            .context("failed to parse VRAM_UsageByFirmware table")?;
    }
    let pp = powerplay::parse_powerplay(&r, pp_off)
        .context("failed to parse PowerPlayInfo table (ATOM_Tonga_POWERPLAYTABLE)")?;
    let vram =
        vram::parse_vram_info(&r, vram_off, mc_off).context("failed to parse VRAM_Info table")?;
    let vrm = if vrm_off != 0 {
        vrm::parse_voltage_object_info(&r, vrm_off)
            .context("failed to parse VoltageObjectInfo table")?
    } else {
        types::VrmInfo {
            fmt_rev: 0,
            cont_rev: 0,
            recognized_format: false,
            objects: Vec::new(),
        }
    };
    let asic = parse_optional(&r, asic_off, "GFX_Info", asic::parse_asic_info)?;
    let smu = parse_optional(&r, smu_off, "SMU_Info", smu::parse_smu_info)?;
    let power_source = parse_optional(
        &r,
        pwr_off,
        "PowerSourceInfo",
        power_source::parse_power_source_info,
    )?;
    let gpio_pin_lut = parse_optional(&r, gpio_off, "GPIO_Pin_LUT", gpio::parse_gpio_pin_lut)?;
    let profiling = parse_optional(
        &r,
        prof_off,
        "ASIC_ProfilingInfo",
        profiling::parse_profiling_info,
    )?;
    let ss = parse_optional(&r, ss_off, "ASIC_InternalSS_Info", ss::parse_ss_info)?;
    let vesa = parse_optional(&r, vesa_off, "StandardVESA_Timing", vesa::parse_vesa_timing)?;
    let i2c = parse_optional(&r, i2c_off, "GPIO_I2C_Info", gpio_i2c::parse_gpio_i2c_info)?;

    let file_name = file_name.to_string();

    let pci_images =
        pci::walk_pci_images(&r).context("failed to walk PCI Option ROM image chain")?;
    let display = display::parse_display_info(&r, rom_header.master_data_table_offset)
        .context("failed to parse video outputs (Object_Header/SupportedDevicesInfo)")?;

    let mut rom = ParsedRom {
        file_name,
        header: rom_header,
        firmware,
        powerplay: pp,
        vram,
        vrm,
        pci_images,
        display,
        asic,
        smu,
        power_source,
        gpio_pin_lut,
        profiling,
        ss,
        vesa,
        i2c,
        warnings: Vec::new(),
    };
    rom.warnings = validate::validate(&rom);
    Ok(rom)
}
