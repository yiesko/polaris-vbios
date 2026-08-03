use super::limits;
use super::types::ParsedRom;

/// Expected compute unit count per device ID, from the physical die:
/// 0x67DF = Polaris 10 (4 SE × 9 CU), 0x67EF = Polaris 11 (2 SE × 8),
/// 0x67FF = Polaris 12 (2 SE × 8), 0x699F = Polaris 12 (2 SE × 5).
/// Used both by `validate` (CUs read from GFX_Info vs die) and by
/// `identify` (die name in the one-line summary).
pub fn die_for_device_id(device_id: u16) -> Option<(&'static str, u32)> {
    match device_id {
        0x67DF => Some(("Polaris 10 (67DF)", 36)),
        0x67EF => Some(("Polaris 11 (67EF)", 16)),
        0x67FF => Some(("Polaris 12 (67FF)", 16)),
        0x699F => Some(("Polaris 12 (699F)", 10)),
        _ => None,
    }
}

/// Normalizes the ASIC name found in the BIOS bootup message to the
/// same family string used by `die_for_device_id`, so both can be
/// compared. Polaris 20/21 are die refreshes of Polaris 10/11 (RX 580 /
/// RX 590 use "Polaris20" even though the device ID is a Polaris 10
/// die), and the marketing names Ellesmere/Baffin/Lexa map 1:1.
fn bootup_asic_family(msg: &str) -> Option<&'static str> {
    let lower = msg.to_ascii_lowercase();
    if lower.contains("polaris10") || lower.contains("ellesmere") || lower.contains("polaris20") {
        Some("Polaris 10")
    } else if lower.contains("polaris11") || lower.contains("baffin") || lower.contains("polaris21")
    {
        Some("Polaris 11")
    } else if lower.contains("polaris12") || lower.contains("lexa") {
        Some("Polaris 12")
    } else {
        None
    }
}

/// Sanity checks on read data. None of these checks prevent reading -
/// they only flag when something deviates from what is expected of a
/// Polaris VBIOS (Tonga/Fiji/Polaris, PowerPlay format 7), whether
/// because the file is from another GPU family, corrupted, or (less
/// likely but possible) there is an offset misalignment not covered
/// by the already validated cases.
pub fn validate(rom: &ParsedRom) -> Vec<String> {
    let mut w = Vec::new();

    if rom.powerplay.header_fmt_rev != 7 {
        w.push(format!(
            "PowerPlay table is in format {}.{}, not 7.x - this usually indicates a GPU \
             outside the Tonga/Fiji/Polaris family (or a pre-GCN3 VBIOS). Values below \
             may not make sense.",
            rom.powerplay.header_fmt_rev, rom.powerplay.header_cont_rev
        ));
    }

    if !rom.header.checksum_valid {
        w.push(format!(
            "ROM checksum does not match (sum = 0x{:02X}, expected 0x00 over {} declared bytes) \
             - the file may be truncated, corrupted, or modified without recalculating the checksum.",
            rom.header.checksum_computed_sum, rom.header.checksum_checked_bytes
        ));
    }

    if (rom.firmware.core_ref_clock_mhz - 100.0).abs() > 5.0 {
        w.push(format!(
            "core reference clock is {:.0} MHz, while the expected value for this family is \
             always 100 MHz - may indicate a misalignment when reading FirmwareInfo.",
            rom.firmware.core_ref_clock_mhz
        ));
    }

    match &rom.powerplay.powertune {
        None => w.push(
            "PowerTune table missing - TDP/TDC/thermal limits could not be read.".to_string(),
        ),
        Some(pt) if pt.tdp_w == 0 => w.push(
            "TDP read as 0 W - the PowerTune table is probably corrupted or empty.".to_string(),
        ),
        Some(pt) => {
            // Envelope per die family (shared with the patch guardrails,
            // see limits.rs): an RX 460 is suspicious at 150 W, an RX
            // 580 "premium" is fine at 185 W - one global range cannot
            // say either. Unrecognized dies keep a coarse sanity net.
            let die = limits::detect_die(rom);
            if die == limits::Die::Unknown {
                if pt.tdp_w > 250 {
                    w.push(format!(
                        "TDP read as {} W - unusually high for a Polaris GPU.",
                        pt.tdp_w
                    ));
                } else if pt.tdp_w < 30 {
                    w.push(format!(
                        "TDP read as {} W - unusually low for a Polaris GPU.",
                        pt.tdp_w
                    ));
                }
            } else {
                match limits::SafeTdp::try_new(pt.tdp_w as u32, die) {
                    Err(e) => w.push(format!(
                        "{} - the ROM may be corrupted or taken from another GPU.",
                        e.message()
                    )),
                    Ok(safe) if safe.is_unusual() => w.push(safe.unusual_message()),
                    _ => {}
                }
            }
        }
    }

    if let Some(pt) = &rom.powerplay.powertune {
        if pt.tjmax_c > 110 {
            w.push(format!(
                "TjMax (edge) read as {} °C - maximum is ~105 °C; this may be a misread.",
                pt.tjmax_c
            ));
        } else if pt.tjmax_c < 60 {
            w.push(format!(
                "TjMax (edge) read as {} °C - unusually low for Polaris.",
                pt.tjmax_c
            ));
        }
    }

    if rom.powerplay.sclk_table.is_empty() {
        w.push("SCLK table (engine clock P-States) came empty.".to_string());
    }
    if rom.powerplay.mclk_table.is_empty() {
        w.push("MCLK table (memory P-States) came empty.".to_string());
    }

    if rom.firmware.bootup_vddc_mv == 0 || rom.firmware.bootup_vddc_mv > 2000 {
        w.push(format!(
            "Boot VDDC read as {} mV, outside plausible range (approx. 700–1200 mV) - \
             check if FirmwareInfo was parsed correctly.",
            rom.firmware.bootup_vddc_mv
        ));
    }

    if rom.vram.modules.is_empty() {
        w.push("no VRAM modules found in the VRAM_Info table.".to_string());
    } else if rom.vram.modules.iter().all(|m| m.part_number.is_empty()) {
        w.push("all VRAM modules came without a part number - may be normal in some ROMs, but worth checking.".to_string());
    }

    if let Some(img0) = rom.pci_images.first() {
        if img0.declared_size_bytes != rom.header.checksum_checked_bytes {
            w.push(format!(
                "image 0 size in the PCIR chain ({} bytes) does not match the size declared by the \
                 legacy checksum ({} bytes) - normally they agree; check if the file was \
                 edited/truncated incorrectly.",
                img0.declared_size_bytes, rom.header.checksum_checked_bytes
            ));
        }
    } else {
        w.push("no valid PCIR image found (not even the legacy one) - this is unexpected for a real VBIOS.".to_string());
    }

    // The BIOS bootup message names its own ASIC ("D00034 Polaris20 XL A1
    // GDDR5 128Mx32 4GB 300e/300m") - if it disagrees with the die the
    // device ID maps to, the ROM was likely taken from another card.
    // Only checked for the unambiguous dies: early production RX 550/560
    // (device 0x67FF/0x699F) legitimately shipped with Baffin/Polaris21
    // bootup strings even though they are Polaris 12 dies.
    if let (Some(msg), Some(img0)) = (&rom.header.bios_bootup_message, rom.pci_images.first())
        && matches!(img0.device_id, 0x67DF | 0x67EF)
        && let Some(boot_family) = bootup_asic_family(msg)
        && let Some((die_family, _)) = die_for_device_id(img0.device_id)
    {
        // die_for_device_id returns "Polaris 10 (67DF)"; compare only
        // the family part, which bootup_asic_family also normalizes to.
        let die_cmp = die_family
            .split_whitespace()
            .take(2)
            .collect::<Vec<_>>()
            .join(" ");
        if boot_family != die_cmp {
            w.push(format!(
                "the BIOS bootup message names the ASIC as \"{boot_family}\", but device ID \
                 0x{:04X} is a {die_family} die - the ROM may have been taken from another GPU.",
                img0.device_id
            ));
        }
    }

    // Compute unit count vs the physical die: GFX_Info reports the
    // maximum layout of the die, so a mismatch means the ROM was
    // written for another GPU (or the fields were misread).
    if let (Some(asic), Some(img0)) = (&rom.asic, rom.pci_images.first()) {
        let total_cus =
            asic.max_shader_engines as u32 * asic.max_sh_per_se as u32 * asic.max_cu_per_sh as u32;
        if let Some((die_name, expected)) = die_for_device_id(img0.device_id)
            && total_cus != expected
        {
            w.push(format!(
                "device ID 0x{:04X} is a {die_name} die, which has {expected} compute units, \
                 but GFX_Info reports {total_cus} CUs - the ROM may have been taken from \
                 another GPU or the ASIC layout fields were misread.",
                img0.device_id
            ));
        }
    }

    // VDDC used by the ROM vs the voltage envelope the die tolerates
    // (ASIC_ProfilingInfo max/min). A boot or LUT voltage above the
    // die maximum is a common sign of a modified/mining ROM.
    if let Some(p) = &rom.profiling {
        let max_vddc_mv = p.max_vddc_mv / 100;
        let boot = rom.firmware.bootup_vddc_mv;
        if boot > 0 && max_vddc_mv > 0 && boot > max_vddc_mv as u16 {
            w.push(format!(
                "boot VDDC is {} mV, above the die maximum of {max_vddc_mv} mV \
                 declared in ASIC_ProfilingInfo - the ROM was likely modified.",
                rom.firmware.bootup_vddc_mv
            ));
        }
        if let Some(lut_max) = rom
            .powerplay
            .vddc_lut
            .iter()
            .filter(|e| e.valid)
            .map(|e| e.vdd_mv)
            .max()
            && max_vddc_mv > 0
            && lut_max > max_vddc_mv as u16
        {
            w.push(format!(
                "highest VDDC LUT entry is {lut_max} mV, above the die maximum of {max_vddc_mv} mV \
                 declared in ASIC_ProfilingInfo; slots above it may be clamped by the driver."
            ));
        }
        if let Some(pt) = &rom.powerplay.powertune
            && let Some(&tdc_dpm0) = p.tdc_limit_per_dpm_a10.first()
        {
            let tdc_a = tdc_dpm0 / 10;
            if tdc_a > 0 && pt.tdc_a > 0 && tdc_a as u16 > pt.tdc_a {
                w.push(format!(
                    "TDC limit of DPM0 in ASIC_ProfilingInfo ({tdc_a} A) exceeds the PowerTune TDC \
                     limit ({} A) - inconsistent table values.",
                    pt.tdc_a
                ));
            }
        }
    }

    w
}

/// INFO-level cross-check of the memory straps / SCLK DPM states /
/// VDDC LUT against the ROM's own Hard Limit table (both data sets are
/// already parsed - this is a free consistency check).
///
/// Severity is deliberately informational, not a validation warning:
/// stock ROMs legitimately trip it (Lenovo and Medion RX560 ship strap
/// clocks above their own hard-limit MCLK). The driver/SMC would clamp
/// to the limit - the ROM boots, it just never uses those clocks.
pub fn cross_checks(rom: &ParsedRom) -> Vec<String> {
    let mut out = Vec::new();
    if rom.powerplay.hard_limits.is_empty() {
        return out;
    }
    for hl in &rom.powerplay.hard_limits {
        let mclk = hl.mclk_limit_mhz;
        if mclk > 0.0 {
            for s in &rom.vram.straps {
                if s.clock_mhz > mclk {
                    out.push(format!(
                        "strap {} MHz exceeds the ROM's own MCLK hard limit of {mclk:.0} MHz",
                        s.clock_mhz
                    ));
                }
            }
            for e in &rom.powerplay.mclk_table {
                if e.mclk_mhz > mclk {
                    out.push(format!(
                        "MCLK DPM level {} ({:.0} MHz) exceeds the ROM's own MCLK hard limit \
                         of {mclk:.0} MHz",
                        e.level, e.mclk_mhz
                    ));
                }
            }
        }
        let sclk = hl.sclk_limit_mhz;
        if sclk > 0.0 {
            for e in &rom.powerplay.sclk_table {
                if e.sclk_mhz > sclk {
                    out.push(format!(
                        "SCLK DPM level {} ({:.0} MHz) exceeds the ROM's own SCLK hard limit \
                         of {sclk:.0} MHz",
                        e.level, e.sclk_mhz
                    ));
                }
            }
        }
        let vddc = hl.vddc_limit_mv;
        if (1..2000).contains(&vddc) {
            for e in rom.powerplay.vddc_lut.iter().filter(|e| e.valid) {
                if e.vdd_mv > vddc {
                    out.push(format!(
                        "VDDC LUT entry {} ({} mV) exceeds the ROM's own VDDC hard limit of \
                         {vddc} mV",
                        e.index, e.vdd_mv
                    ));
                }
            }
        }
    }
    out
}
