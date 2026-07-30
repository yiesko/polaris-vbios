use anyhow::{Result, bail};

use super::reader::Reader;
use super::types::RomHeader;

/// Names of the 35 entries in `ATOM_MASTER_LIST_OF_DATA_TABLES`, in the
/// exact order defined in atombios.h (order matters: this maps each
/// index to the correct offset inside the Master Data Table).
pub const MASTER_TABLE_NAMES: [&str; 35] = [
    "UtilityPipeLine",
    "MultimediaCapabilityInfo",
    "MultimediaConfigInfo",
    "StandardVESA_Timing",
    "FirmwareInfo",
    "PaletteData",
    "LCD_Info",
    "DIGTransmitterInfo",
    "SMU_Info",
    "SupportedDevicesInfo",
    "GPIO_I2C_Info",
    "VRAM_UsageByFirmware",
    "GPIO_Pin_LUT",
    "VESA_ToInternalModeLUT",
    "GFX_Info",
    "PowerPlayInfo",
    "GPUVirtualizationInfo",
    "SaveRestoreInfo",
    "PPLL_SS_Info",
    "OemInfo",
    "XTMDS_Info",
    "MclkSS_Info",
    "Object_Header",
    "IndirectIOAccess",
    "MC_InitParameter",
    "ASIC_VDDC_Info",
    "ASIC_InternalSS_Info",
    "TV_VideoMode",
    "VRAM_Info",
    "MemoryTrainingInfo",
    "IntegratedSystemInfo",
    "ASIC_ProfilingInfo",
    "VoltageObjectInfo",
    "PowerSourceInfo",
    "ServiceInfo",
];

pub fn table_index(name: &str) -> usize {
    MASTER_TABLE_NAMES
        .iter()
        .position(|&n| n == name)
        .unwrap_or_else(|| panic!("unknown table: {name}"))
}

/// Names of the 81 entries in `ATOM_MASTER_LIST_OF_COMMAND_TABLES`, in
/// the exact order defined in atombios.h (line 272).
pub const COMMAND_TABLE_NAMES: [&str; 81] = [
    "ASIC_Init",
    "GetDisplaySurfaceSize",
    "ASIC_RegistersInit",
    "VRAM_BlockVenderDetection",
    "DIGxEncoderControl",
    "MemoryControllerInit",
    "EnableCRTCMemReq",
    "MemoryParamAdjust",
    "DVOEncoderControl",
    "GPIOPinControl",
    "SetEngineClock",
    "SetMemoryClock",
    "SetPixelClock",
    "EnableDispPowerGating",
    "ResetMemoryDLL",
    "ResetMemoryDevice",
    "MemoryPLLInit",
    "AdjustDisplayPll",
    "AdjustMemoryController",
    "EnableASIC_StaticPwrMgt",
    "SetUniphyInstance",
    "DAC_LoadDetection",
    "LVTMAEncoderControl",
    "HW_Misc_Operation",
    "DAC1EncoderControl",
    "DAC2EncoderControl",
    "DVOOutputControl",
    "CV1OutputControl",
    "GetConditionalGoldenSetting",
    "SMC_Init",
    "PatchMCSetting",
    "MC_SEQ_Control",
    "Gfx_Harvesting",
    "EnableScaler",
    "BlankCRTC",
    "EnableCRTC",
    "GetPixelClock",
    "EnableVGA_Render",
    "GetSCLKOverMCLKRatio",
    "SetCRTC_Timing",
    "SetCRTC_OverScan",
    "GetSMUClockInfo",
    "SelectCRTC_Source",
    "EnableGraphSurfaces",
    "UpdateCRTC_DoubleBufferRegisters",
    "LUT_AutoFill",
    "SetDCEClock",
    "GetMemoryClock",
    "GetEngineClock",
    "SetCRTC_UsingDTDTiming",
    "ExternalEncoderControl",
    "LVTMAOutputControl",
    "VRAM_BlockDetectionByStrap",
    "MemoryCleanUp",
    "ProcessI2cChannelTransaction",
    "WriteOneByteToHWAssistedI2C",
    "ReadHWAssistedI2CStatus",
    "SpeedFanControl",
    "PowerConnectorDetection",
    "MC_Synchronization",
    "ComputeMemoryEnginePLL",
    "Gfx_Init",
    "VRAM_GetCurrentInfoBlock",
    "DynamicMemorySettings",
    "MemoryTraining",
    "EnableSpreadSpectrumOnPPLL",
    "TMDSAOutputControl",
    "SetVoltage",
    "DAC1OutputControl",
    "ReadEfuseValue",
    "ComputeMemoryClockParam",
    "ClockSource",
    "MemoryDeviceInit",
    "GetDispObjectInfo",
    "DIG1EncoderControl",
    "DIG2EncoderControl",
    "DIG1TransmitterControl",
    "DIG2TransmitterControl",
    "ProcessAuxChannelTransaction",
    "DPEncoderService",
    "GetVoltageInfo",
];

/// Names of the command tables present (offset != 0) in the
/// `ATOM_MASTER_LIST_OF_COMMAND_TABLES`.
fn parse_command_tables(r: &Reader, mct_offset: usize) -> Vec<String> {
    if mct_offset == 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (idx, name) in COMMAND_TABLE_NAMES.iter().enumerate() {
        // 4 bytes of ATOM_COMMON_TABLE_HEADER before the ushort array.
        if let Ok(off) = r.u16(mct_offset + 4 + idx * 2)
            && off != 0
        {
            out.push((*name).to_string());
        }
    }
    out
}

pub fn parse_rom_header(r: &Reader) -> Result<RomHeader> {
    if r.len() < 2 || r.u8(0)? != 0x55 || r.u8(1)? != 0xAA {
        bail!("legacy BIOS signature missing (expected 0x55 0xAA at start of file)");
    }

    // Pointer to the ATOM header is always at fixed offset 0x48.
    let atom_ptr = r.u16(0x48)? as usize;
    let sig = r.bytes(atom_ptr + 4, 4)?;
    if sig != b"ATOM" {
        bail!(
            "signature 'ATOM' not found at 0x{:X} (found: {:?}) - this file does not appear to be a valid AtomBIOS",
            atom_ptr,
            String::from_utf8_lossy(sig)
        );
    }

    let atom_struct_size = r.u16(atom_ptr)?;
    let atom_fmt_rev = r.u8(atom_ptr + 2)?;
    let atom_cont_rev = r.u8(atom_ptr + 3)?;
    // ATOM_ROM_HEADER: usMasterCommandTableOffset at +0x1e, usMasterDataTableOffset at +0x20
    let master_cmd_table_offset = r.u16(atom_ptr + 0x1e)? as usize;
    let master_data_table_offset = r.u16(atom_ptr + 0x20)? as usize;

    // usSubsystemVendorID at +0x18, usSubsystemID at +0x1a
    let subsystem_vendor_id = r.u16(atom_ptr + 0x18)?;
    let subsystem_device_id = r.u16(atom_ptr + 0x1a)?;
    let subsystem_vendor_name = subsystem_vendor_name(subsystem_vendor_id).map(str::to_string);

    let checksum = compute_checksum(r)?;
    let bios_bootup_message = parse_bios_bootup_message(r, atom_ptr);
    let config_filename = parse_config_filename(r, atom_ptr);
    let build_date_candidates = scan_date_strings(r, 2048);
    let command_tables_present = parse_command_tables(r, master_cmd_table_offset);

    Ok(RomHeader {
        file_size: r.len(),
        atom_header_offset: atom_ptr,
        atom_struct_size,
        atom_fmt_rev,
        atom_cont_rev,
        master_data_table_offset,
        master_cmd_table_offset,
        subsystem_vendor_id,
        subsystem_device_id,
        subsystem_vendor_name,
        checksum_declared_size_blocks: checksum.0,
        checksum_checked_bytes: checksum.1,
        checksum_computed_sum: checksum.2,
        checksum_valid: checksum.3,
        bios_bootup_message,
        config_filename,
        build_date_candidates,
        command_tables_present,
    })
}

/// Known PCI subsystem vendor IDs for board manufacturers - list
/// checked against public AMD documentation (not exhaustive; a
/// missing ID here does not mean anything is wrong with the ROM,
/// it just is not in this small catalog).
fn subsystem_vendor_name(id: u16) -> Option<&'static str> {
    Some(match id {
        0x1002 => "AMD/ATI (reference board)",
        0x1043 => "ASUSTeK",
        0x196D => "Club 3D",
        0x1092 => "Diamond Multimedia",
        0x18BC => "GeCube",
        0x1458 => "Gigabyte",
        0x17AF => "HIS",
        0x16F3 => "Jetway",
        0x1462 => "MSI",
        0x174B => "Sapphire / PC Partner",
        0x148C => "PowerColor",
        0x1545 => "VisionTek",
        0x1682 => "XFX",
        0x1025 => "Acer",
        0x1028 => "Dell",
        0x103C => "HP",
        0x17AA => "Lenovo",
        _ => return None,
    })
}

/// Classic legacy option ROM checksum: the sum of all bytes in the
/// declared region (byte 2 × 512) must wrap to 0x00 mod 256.
/// Returns (declared blocks, checked bytes, computed sum, valid?).
fn compute_checksum(r: &Reader) -> Result<(u8, usize, u8, bool)> {
    let size_blocks = r.u8(2)?;
    let declared = size_blocks as usize * 512;
    let checked = declared.min(r.len());
    let mut sum: u8 = 0;
    for i in 0..checked {
        sum = sum.wrapping_add(r.u8(i)?);
    }
    let valid = checked > 0 && sum == 0;
    Ok((size_blocks, checked, sum, valid))
}

/// Reads the official "BIOS bootup message" string (pointed to by the
/// usBIOS_BootupMessageOffset field in ATOM_ROM_HEADER, offset +0x10) -
/// the card's native self-description printed at POST (internal board
/// ID, ASIC name/revision, memory config, boot clocks). The message is
/// usually preceded by CR/LF (it is printed as a new line) and padded
/// with spaces; both are stripped here.
fn parse_bios_bootup_message(r: &Reader, atom_ptr: usize) -> Option<String> {
    let off = r.u16(atom_ptr + 0x10).ok()? as usize;
    if off == 0 {
        return None;
    }
    let raw = r.bytes(off, 128).ok()?;
    let start = raw
        .iter()
        .position(|&b| b != 0 && !b.is_ascii_control())
        .unwrap_or(raw.len());
    let rest = &raw[start..];
    let end = rest
        .iter()
        .position(|&b| b == 0 || b == 0x0d || b == 0x0a)
        .unwrap_or(rest.len());
    let s = String::from_utf8_lossy(&rest[..end]).trim().to_string();
    if s.is_empty() || s.len() < 4 || !s.bytes().all(|b| b.is_ascii_graphic() || b == b' ') {
        None
    } else {
        Some(s)
    }
}

/// Reads the internal config file name of the BIOS build (pointed to by
/// usConfigFilenameOffset, offset +0x0c) - e.g. "343L0506.S07" on a
/// Sapphire RX 570. The name of the file the BIOS was built from, in
/// the board vendor's naming scheme; padded with spaces to a fixed
/// width inside the ROM.
fn parse_config_filename(r: &Reader, atom_ptr: usize) -> Option<String> {
    let off = r.u16(atom_ptr + 0x0c).ok()? as usize;
    if off == 0 {
        return None;
    }
    let s = r.cstr(off, 24).ok()?.trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

/// Scans the first `scan_len` bytes for printable ASCII runs that
/// contain a date pattern (dd/dd/dd or dd/dd/dddd). This is a heuristic
/// — there is no official AtomBIOS field guaranteed to be a "build
/// date"; many VBIOS store this information as a loose string near
/// the header.
fn scan_date_strings(r: &Reader, scan_len: usize) -> Vec<String> {
    let n = scan_len.min(r.len());
    let mut runs = Vec::new();
    let mut run = String::new();
    for i in 0..n {
        let b = r.u8(i).unwrap_or(0);
        if b.is_ascii_graphic() || b == b' ' {
            run.push(b as char);
        } else {
            if run.len() >= 6 {
                runs.push(run.clone());
            }
            run.clear();
        }
    }
    if run.len() >= 6 {
        runs.push(run);
    }

    let mut out: Vec<String> = runs
        .into_iter()
        .filter(|s| contains_date_pattern(s))
        .map(|s| s.trim().to_string())
        .collect();
    out.dedup();
    out.truncate(3);
    out
}

fn contains_date_pattern(s: &str) -> bool {
    let b = s.as_bytes();
    for start in 0..b.len() {
        if match_date_at(b, start).is_some() {
            return true;
        }
    }
    false
}

fn match_date_at(b: &[u8], start: usize) -> Option<usize> {
    let mut i = start;
    take_digits(b, &mut i, 1, 2)?;
    if i >= b.len() || b[i] != b'/' {
        return None;
    }
    i += 1;
    take_digits(b, &mut i, 1, 2)?;
    if i >= b.len() || b[i] != b'/' {
        return None;
    }
    i += 1;
    take_digits(b, &mut i, 2, 4)?;
    Some(i - start)
}

fn take_digits(b: &[u8], i: &mut usize, min: usize, max: usize) -> Option<usize> {
    let start = *i;
    while *i < b.len() && b[*i].is_ascii_digit() && *i - start < max {
        *i += 1;
    }
    let n = *i - start;
    if n < min { None } else { Some(n) }
}

/// Reads the 35 offsets (absolute, from the start of the ROM) in the
/// Master Data Table and returns the offset of a specific table by name.
pub fn master_table_offset(r: &Reader, mdt_offset: usize, name: &str) -> Result<usize> {
    let idx = table_index(name);
    // 4 bytes of ATOM_COMMON_TABLE_HEADER before the ushort array.
    let off = r.u16(mdt_offset + 4 + idx * 2)? as usize;
    Ok(off)
}

/// Size in bytes of the Master Data Table layout region (4 bytes of
/// common table header + one ushort per data table entry). Every byte
/// here decides where a data table lives - `patch` treats it as
/// protected layout.
pub fn master_data_table_layout_size() -> usize {
    4 + MASTER_TABLE_NAMES.len() * 2
}

/// Same, for the Master List of Command Tables (4 bytes of common table
/// header + one ushort per command table entry).
pub fn master_cmd_table_layout_size() -> usize {
    4 + COMMAND_TABLE_NAMES.len() * 2
}
