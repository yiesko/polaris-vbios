pub mod asic;
pub mod disasm;
pub mod display;
pub mod firmware;
pub mod gpio;
pub mod gpio_i2c;
pub mod header;
pub mod locate;
pub mod patch;
pub mod pci;
pub mod power_source;
pub mod powerplay;
pub mod profiling;
pub mod reader;
pub mod smu;
pub mod ss;
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
    let asic = if asic_off != 0 {
        Some(asic::parse_asic_info(&r, asic_off).context("failed to parse GFX_Info table")?)
    } else {
        None
    };
    let smu = if smu_off != 0 {
        Some(smu::parse_smu_info(&r, smu_off).context("failed to parse SMU_Info table")?)
    } else {
        None
    };
    let power_source = if pwr_off != 0 {
        Some(
            power_source::parse_power_source_info(&r, pwr_off)
                .context("failed to parse PowerSourceInfo table")?,
        )
    } else {
        None
    };
    let gpio_pin_lut = if gpio_off != 0 {
        Some(gpio::parse_gpio_pin_lut(&r, gpio_off).context("failed to parse GPIO_Pin_LUT table")?)
    } else {
        None
    };
    let profiling = if prof_off != 0 {
        Some(
            profiling::parse_profiling_info(&r, prof_off)
                .context("failed to parse ASIC_ProfilingInfo table")?,
        )
    } else {
        None
    };
    let ss = if ss_off != 0 {
        Some(ss::parse_ss_info(&r, ss_off).context("failed to parse ASIC_InternalSS_Info table")?)
    } else {
        None
    };
    let vesa = if vesa_off != 0 {
        Some(
            vesa::parse_vesa_timing(&r, vesa_off)
                .context("failed to parse StandardVESA_Timing table")?,
        )
    } else {
        None
    };
    let i2c = if i2c_off != 0 {
        Some(
            gpio_i2c::parse_gpio_i2c_info(&r, i2c_off)
                .context("failed to parse GPIO_I2C_Info table")?,
        )
    } else {
        None
    };

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

#[cfg(test)]
pub(crate) mod test_support {
    use std::path::PathBuf;

    /// Collects the `.rom` files under `samples/BIOS/<family>/`, or
    /// `None` when the directory is absent (tests skip in that case).
    /// Shared by the locate and validate sample sweeps.
    pub fn sample_roms() -> Option<Vec<PathBuf>> {
        let dir = std::fs::read_dir("samples/BIOS").ok()?;
        Some(
            dir.filter_map(|e| e.ok())
                .flat_map(|e| std::fs::read_dir(e.path()).ok())
                .flatten()
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "rom"))
                .collect(),
        )
    }
}
