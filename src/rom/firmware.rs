use anyhow::Result;

use super::reader::Reader;
use super::types::{FirmwareInfo, FirmwareVramReserve};

/// ucCoolingSolution_ID (atombios.h V2_2 comment; enum
/// atom_cooling_solution_id in the kernel's atomfirmware.h).
fn cooling_solution_name(id: u8) -> &'static str {
    match id {
        0x00 => "air cooling",
        0x01 => "liquid cooling",
        _ => "unknown",
    }
}

/// Parses `ATOM_FIRMWARE_INFO_V2_2` (108 bytes, format 2.2 - the one
/// used by Polaris). Offsets checked byte by byte against the real
/// struct from the Linux kernel (drivers/gpu/drm/amd/include/atombios.h).
/// Note: fields marked "Was ..." in the header are reserved in V2_2 and
/// are not read here.
pub fn parse_firmware_info(r: &Reader, off: usize) -> Result<FirmwareInfo> {
    let struct_size = r.u16(off)?;
    let fmt_rev = r.u8(off + 2)?;
    let cont_rev = r.u8(off + 3)?;

    let firmware_revision = r.u32(off + 4)?;
    let default_engine_clock_10khz = r.u32(off + 8)?;
    let default_memory_clock_10khz = r.u32(off + 12)?;
    let bootup_vddc_mv = r.u16(off + 46)?;
    let bootup_vddci_mv = if struct_size >= 80 {
        r.u16(off + 78)?
    } else {
        0
    };
    let core_ref_clock_10khz = if struct_size >= 84 {
        r.u16(off + 82)?
    } else {
        0
    };
    let mem_ref_clock_10khz = if struct_size >= 86 {
        r.u16(off + 84)?
    } else {
        0
    };
    let memory_module_id = if struct_size >= 89 {
        r.u8(off + 88)?
    } else {
        0
    };
    let bootup_mvddc_mv = if struct_size >= 94 {
        r.u16(off + 92)?
    } else {
        0
    };
    let bootup_vddgfx_mv = if struct_size >= 96 {
        r.u16(off + 94)?
    } else {
        0
    };

    // V2_2 extended fields (all in 10 kHz except where noted).
    let spll_output_10khz = if struct_size >= 24 {
        r.u32(off + 16)?
    } else {
        0
    };
    let gpull_output_10khz = if struct_size >= 24 {
        r.u32(off + 20)?
    } else {
        0
    };
    let max_pixel_clock_pll_10khz = if struct_size >= 36 {
        r.u32(off + 32)?
    } else {
        0
    };
    let default_disp_engine_10khz = if struct_size >= 44 {
        r.u32(off + 40)?
    } else {
        0
    };
    let min_pixel_clock_pll_output_10khz = if struct_size >= 60 {
        r.u32(off + 56)?
    } else {
        0
    };
    let min_pixel_clock_pll_input_10khz = if struct_size >= 78 {
        r.u16(off + 74)?
    } else {
        0
    };
    let max_pixel_clock_pll_input_10khz = if struct_size >= 78 {
        r.u16(off + 76)?
    } else {
        0
    };
    let uniphy_dp_mode_ext_10khz = if struct_size >= 88 {
        r.u16(off + 86)?
    } else {
        0
    };
    let cooling_solution_id = if struct_size >= 90 {
        r.u8(off + 89)?
    } else {
        0
    };
    let product_branding = if struct_size >= 91 {
        r.u8(off + 90)?
    } else {
        0
    };

    Ok(FirmwareInfo {
        struct_size,
        fmt_rev,
        cont_rev,
        firmware_revision,
        default_engine_clock_mhz: default_engine_clock_10khz as f64 / 100.0,
        default_memory_clock_mhz: default_memory_clock_10khz as f64 / 100.0,
        core_ref_clock_mhz: core_ref_clock_10khz as f64 / 100.0,
        mem_ref_clock_mhz: mem_ref_clock_10khz as f64 / 100.0,
        bootup_vddc_mv,
        bootup_vddci_mv,
        bootup_mvddc_mv,
        bootup_vddgfx_mv,
        memory_module_id,
        spll_output_mhz: spll_output_10khz as f64 / 100.0,
        gpull_output_mhz: gpull_output_10khz as f64 / 100.0,
        max_pixel_clock_pll_mhz: max_pixel_clock_pll_10khz as f64 / 100.0,
        default_disp_engine_clk_mhz: default_disp_engine_10khz as f64 / 100.0,
        min_pixel_clock_pll_input_mhz: min_pixel_clock_pll_input_10khz as f64 / 100.0,
        max_pixel_clock_pll_input_mhz: max_pixel_clock_pll_input_10khz as f64 / 100.0,
        min_pixel_clock_pll_output_mhz: min_pixel_clock_pll_output_10khz as f64 / 100.0,
        uniphy_dp_mode_ext_clk_mhz: uniphy_dp_mode_ext_10khz as f64 / 100.0,
        cooling_solution_id,
        cooling_solution_name: cooling_solution_name(cooling_solution_id).to_string(),
        // PRODUCT_BRANDING (atombios.h line 3137): bits[7:4] branding ID,
        // bits[1:0] embedded feature level.
        branding_id: product_branding >> 4,
        embedded_cap: product_branding & 0x03,
        vram_reserves: Vec::new(),
    })
}

/// Parses the VRAM_UsageByFirmware table (`ATOM_VRAM_USAGE_BY_FIRMWARE`
/// / `_V1_5`, atombios.h line 4338): the VRAM regions the BIOS
/// firmware and the driver reserve at boot. Format 1.5 uses
/// `ATOM_FIRMWARE_VRAM_RESERVE_INFO_V1_5` (start + two KiB sizes);
/// older format 1.0/1.1 entries have a different tail (only
/// `usFirmwareUseInKb`), so only 1.5 is decoded.
pub fn parse_vram_usage(r: &Reader, off: usize) -> Result<Vec<FirmwareVramReserve>> {
    let struct_size = r.u16(off)?;
    let cont_rev = r.u8(off + 3)?;
    if cont_rev < 5 {
        return Ok(Vec::new());
    }
    let avail = (struct_size as usize).saturating_sub(4) / 8;
    let mut out = Vec::with_capacity(avail);
    for i in 0..avail {
        let p = off + 4 + i * 8;
        out.push(FirmwareVramReserve {
            start_addr: r.u32(p)?,
            firmware_use_kb: r.u16(p + 4)?,
            driver_use_kb: r.u16(p + 6)?,
        });
    }
    Ok(out)
}
