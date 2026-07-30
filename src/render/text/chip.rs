use super::{kv, optional_heading};
use crate::render::color::Palette;
use crate::render::sections::Section;
use crate::rom::types::{GpioI2cAssignment, ParsedRom};

/// `ATOM_GFX_INFO` - physical ASIC layout (CU/SE/RB counts).
pub(super) fn render_asic(rom: &ParsedRom, pal: &Palette) -> String {
    let Some(a) = &rom.asic else {
        return optional_heading(pal, Section::Asic, false);
    };
    let mut s = optional_heading(pal, Section::Asic, true);
    s.push('\n');
    s.push_str(&format!(
        "  {} GFX IP {}.{}\n",
        pal.label("GFX IP version"),
        a.gfx_ip_maj_ver,
        a.gfx_ip_min_ver
    ));
    s.push_str(&format!(
        "  {} {}\n",
        pal.label("Shader engines (SE)"),
        a.max_shader_engines
    ));
    s.push_str(&format!(
        "  {} {}\n",
        pal.label("Shader arrays per SE"),
        a.max_sh_per_se
    ));
    s.push_str(&format!(
        "  {} {}\n",
        pal.label("Compute units per SH"),
        a.max_cu_per_sh
    ));
    let total_cus = a.max_shader_engines as u32 * a.max_sh_per_se as u32 * a.max_cu_per_sh as u32;
    s.push_str(&format!(
        "  {} {}\n",
        pal.label("Total CUs (max)"),
        total_cus
    ));
    s.push_str(&format!(
        "  {} {}\n",
        pal.label("Render backends per SE"),
        a.max_backends_per_se
    ));
    s.push_str(&format!(
        "  {} {}\n",
        pal.label("Tile pipes"),
        a.max_tile_pipes
    ));
    s.push_str(&format!(
        "  {} {}\n",
        pal.label("Texture channel caches"),
        a.max_texture_channel_caches
    ));
    if let Some(lt) = a.hi_lo_leakage_threshold {
        s.push_str(&format!(
            "  {} 0x{:X}\n",
            pal.label("Hi/Lo leakage threshold"),
            lt
        ));
    }
    s
}

/// `ATOM_SMU_INFO_V2_1` - SMU firmware version and SCLK FCW ranges.
pub(super) fn render_smu(rom: &ParsedRom, pal: &Palette) -> String {
    let Some(smu) = &rom.smu else {
        return optional_heading(pal, Section::Smu, false);
    };
    let mut s = optional_heading(pal, Section::Smu, true);
    s.push('\n');
    s.push_str(&kv(pal, "SMU firmware version", smu.smu_ver));
    s.push('\n');
    s.push_str(&kv(pal, "Share power source", smu.share_power_source));
    if smu.fcw_entries.is_empty() {
        s.push_str("\n  (no SCLK FCW range entries)");
        return s;
    }
    for e in &smu.fcw_entries {
        s.push('\n');
        s.push_str(&format!(
            "  {} range {} - max {:.0} MHz · {} · postdiv 2^{}\n",
            pal.value("▸"),
            e.index,
            e.max_sclk_mhz,
            e.vco_setting_name,
            e.postdiv
        ));
        s.push_str(&format!(
            "      FCW: pcc {} · trans upper {} · rcw trans lower {}\n",
            e.fcw_pcc, e.fcw_trans_upper, e.rcw_trans_lower
        ));
    }
    s
}

/// `ATOM_POWER_SOURCE_INFO` - power inputs (PCIe slot / 6-pin / 8-pin)
/// and how the VBIOS detects them.
pub(super) fn render_power(rom: &ParsedRom, pal: &Palette) -> String {
    let Some(ps) = &rom.power_source else {
        return optional_heading(pal, Section::Power, false);
    };
    let mut s = optional_heading(pal, Section::Power, true);
    if ps.objects.is_empty() {
        s.push_str("\n  (no power source objects)");
        return s;
    }
    for obj in &ps.objects {
        s.push('\n');
        s.push_str(&format!(
            "  {} {}",
            pal.value("▸"),
            pal.good(&obj.source_name)
        ));
        if obj.sensed_power_w > 0 {
            s.push_str(&format!(" - sensed {} W", obj.sensed_power_w));
        }
        s.push('\n');
        s.push_str(&format!("      detection: {}\n", obj.sensor_type_name));
        if obj.sensor_type_raw == 2 {
            s.push_str(&format!(
                "      i2c id {} · slave addr 0x{:02X} · reg index 0x{:02X} · bitmask 0x{:02X} · active state {}\n",
                obj.sensor_id, obj.sensor_slave_addr, obj.sensor_reg_index,
                obj.sensor_reg_bit_mask, obj.sensor_active_state
            ));
        } else if obj.sensor_type_raw == 1 {
            s.push_str(&format!(
                "      gpio id {} · active state {}\n",
                obj.sensor_id, obj.sensor_active_state
            ));
        }
    }
    s
}

/// `ATOM_GPIO_PIN_LUT` - predefined GPIO pin roles.
pub(super) fn render_gpio(rom: &ParsedRom, pal: &Palette) -> String {
    let Some(lut) = &rom.gpio_pin_lut else {
        return optional_heading(pal, Section::Gpio, false);
    };
    let mut s = optional_heading(pal, Section::Gpio, true);
    if lut.pins.is_empty() {
        s.push_str("\n  (no GPIO pin assignments)");
        return s;
    }
    for pin in &lut.pins {
        let name = pin.gpio_id_name.as_deref().unwrap_or("reserved/unused");
        s.push_str(&format!(
            "\n  {} pin id {} (0x{:02X}) · GPIO index {} bit shift {} · {}",
            pal.value("▸"),
            pin.gpio_id,
            pin.gpio_id,
            pin.gpio_pin_a_index,
            pin.pin_bit_shift,
            pal.good(name)
        ));
    }
    s
}

/// `ATOM_ASIC_PROFILING_INFO_V3_6` - die voltage envelope, efuse
/// addresses and per-DPM current limits.
pub(super) fn render_profiling(rom: &ParsedRom, pal: &Palette) -> String {
    let Some(p) = &rom.profiling else {
        return optional_heading(pal, Section::Profiling, false);
    };
    let mut s = optional_heading(pal, Section::Profiling, true);
    s.push('\n');
    s.push_str(&kv(
        pal,
        "Max VDDC (die limit)",
        format!("{} mV", p.max_vddc_mv / 100),
    ));
    s.push('\n');
    s.push_str(&kv(
        pal,
        "Min VDDC (die limit)",
        format!("{} mV", p.min_vddc_mv / 100),
    ));
    s.push('\n');
    s.push_str(&kv(
        pal,
        "Max voltage (0.25 mV units)",
        format!(
            "{} ({} mV)",
            p.max_voltage_0_25mv,
            p.max_voltage_0_25mv as u32 * 25 / 100
        ),
    ));
    s.push('\n');
    s.push_str(&kv(pal, "Load line slope", format!("{}", p.load_line_slop)));
    s.push('\n');
    s.push_str(&format!(
        "  {} leakage efuse · dword idx {} · bit lsb {} · length {}\n",
        pal.label("Leakage efuse"),
        p.lkg_euse_index,
        p.lkg_efuse_bit_lsb,
        p.lkg_efuse_length
    ));
    s.push_str(&format!(
        "  {} dword idx {} · bit lsb {} · length {} · encode range {} · min {}\n",
        pal.label("RO (ring osc) efuse"),
        p.ro_fuse.efuse_index,
        p.ro_fuse.efuse_bit_lsb,
        p.ro_fuse.efuse_length,
        p.ro_fuse.efuse_encode_range,
        p.ro_fuse.efuse_min
    ));
    s.push('\n');
    s.push_str(&kv(
        pal,
        "EVV default VDDC",
        format!("{:.3} V", p.evv_default_vddc_v100000 as f64 / 100000.0),
    ));
    s.push('\n');
    s.push_str(&kv(
        pal,
        "EVV no-calc VDDC",
        format!("{:.3} V", p.evv_no_calc_vddc_v100000 as f64 / 100000.0),
    ));
    if !p.tdc_limit_per_dpm_a10.is_empty() {
        s.push('\n');
        s.push_str(&format!(
            "  {} (0.1 A per unit)\n",
            pal.label("TDC limit per DPM")
        ));
        for (i, tdc) in p.tdc_limit_per_dpm_a10.iter().enumerate() {
            s.push_str(&format!(
                "    {} DPM{}: {} A\n",
                pal.value("▸"),
                i,
                tdc / 10
            ));
        }
    }
    if !p.no_calc_vddc_per_dpm_v1000000.is_empty() {
        s.push('\n');
        s.push_str(&format!(
            "  {} (VDDC to use when EVV fails)\n",
            pal.label("No-calc VDDC per DPM")
        ));
        for (i, v) in p.no_calc_vddc_per_dpm_v1000000.iter().enumerate() {
            s.push_str(&format!(
                "    {} DPM{}: {} mV\n",
                pal.value("▸"),
                i,
                v / 1000
            ));
        }
    }
    s.push('\n');
    s.push_str(&format!(
        "  {} GB droop: cksoff {} · ckson {} · fuse cksoff {} · fuse ckson {}\n",
        pal.label("AVFS enables"),
        p.enable_gb_vdroop_cksoff,
        p.enable_gb_vdroop_ckson,
        p.enable_gb_fuse_cksoff,
        p.enable_gb_fuse_ckson
    ));
    s
}

/// `ATOM_ASIC_INTERNAL_SS_INFO_V3` - spread spectrum of the internal
/// clock generators (memory/engine/DP/...).
pub(super) fn render_ss(rom: &ParsedRom, pal: &Palette) -> String {
    let Some(ss) = &rom.ss else {
        return optional_heading(pal, Section::Ss, false);
    };
    let mut s = optional_heading(pal, Section::Ss, true);
    if ss.entries.is_empty() {
        s.push_str("\n  (no spread spectrum entries)");
        return s;
    }
    for e in &ss.entries {
        let mode_bits = [
            ("down spread", e.spread_mode & 0x01 == 0),
            ("centre spread", e.spread_mode & 0x01 != 0),
            ("internal", e.spread_mode & 0x02 == 0),
            ("external", e.spread_mode & 0x02 != 0),
        ];
        let mode_str: Vec<&str> = mode_bits
            .iter()
            .filter(|(_, on)| *on)
            .map(|(name, _)| *name)
            .collect();
        let target = if e.target_clock_range_khz10 == 0x00FF_FFFF {
            "all clocks".to_string()
        } else {
            format!("{} MHz", e.target_clock_range_khz10 as f64 / 100.0)
        };
        s.push_str(&format!(
            "\n  {} {} · target {} · {:.2}% · {} kHz · mode 0x{:02X} ({})",
            pal.value("▸"),
            pal.good(crate::rom::ss::ss_clock_name(e.clock_indication)),
            target,
            e.spread_pct,
            e.spread_rate_hz10 as f64 / 100.0,
            e.spread_mode,
            mode_str.join(", ")
        ));
    }
    s
}

/// `ATOM_STANDARD_VESA_TIMING` - native VESA DMT modes the VBIOS can
/// drive without a display driver.
pub(super) fn render_vesa(rom: &ParsedRom, pal: &Palette) -> String {
    let Some(vesa) = &rom.vesa else {
        return optional_heading(pal, Section::Vesa, false);
    };
    let mut s = optional_heading(pal, Section::Vesa, true);
    if vesa.modes.is_empty() {
        s.push_str("\n  (no native VESA modes)");
        return s;
    }
    s.push_str(&format!(
        "\n  {} rev {}.{} · {} modes\n",
        pal.label("StandardVESA_Timing"),
        vesa.fmt_rev,
        vesa.cont_rev,
        vesa.modes.len()
    ));
    for m in &vesa.modes {
        let res = format!("{}x{}", m.h_active, m.v_active);
        s.push_str(&format!(
            "\n  {} mode {} · {} @ {:.2} Hz · pix {} MHz · hblk {} hsync {}/{} · vblk {} vsync {}/{} · {} · int {}",
            pal.value("▸"),
            m.index,
            pal.good(&res),
            m.refresh_rate_hz,
            m.pixel_clock_mhz,
            m.h_blanking,
            m.h_sync_offset,
            m.h_sync_width,
            m.v_blanking,
            m.v_sync_offset,
            m.v_sync_width,
            m.sync_polarity,
            m.internal_mode_number,
        ));
    }
    s
}

/// `ATOM_GPIO_I2C_INFO` - the board's I2C/DDC line wiring: which GPIO
/// registers drive each line and at which bit positions.
pub(super) fn render_i2c(rom: &ParsedRom, pal: &Palette) -> String {
    let Some(i2c) = &rom.i2c else {
        return optional_heading(pal, Section::I2c, false);
    };
    let mut s = optional_heading(pal, Section::I2c, true);
    if i2c.assignments.is_empty() {
        s.push_str("\n  (no I2C line assignments)");
        return s;
    }
    s.push_str(&format!(
        "\n  {} rev {}.{} · {} lines\n",
        pal.label("GPIO_I2C_Info"),
        i2c.fmt_rev,
        i2c.cont_rev,
        i2c.assignments.len()
    ));
    for a in &i2c.assignments {
        let hw_capable = a.i2c_id & 0x80 != 0;
        let engine_id = (a.i2c_id >> 4) & 0x07;
        let line_mux = a.i2c_id & 0x0F;
        let engine = if hw_capable {
            match engine_id {
                2 => "multimedia hw engine".to_string(),
                1 => "hw engine".to_string(),
                _ => format!("hw engine {}", engine_id),
            }
        } else {
            "SW bit-banged".to_string()
        };
        let mask = |shift: u8| {
            if shift < 32 {
                format!("0x{:02X}", GpioI2cAssignment::bit_mask(shift))
            } else {
                "n/a".to_string()
            }
        };
        s.push_str(&format!(
            "\n  {} line 0x{:02X} (mux {} · {}) · {}",
            pal.value("▸"),
            a.i2c_id,
            line_mux,
            engine,
            pal.good(&format!(
                "clk regs 0x{:04X}/0x{:04X}/0x{:04X}/0x{:04X} (mask/en/y/a, bits {}/{}/{}/{})",
                a.clk_mask_reg,
                a.clk_en_reg,
                a.clk_y_reg,
                a.clk_a_reg,
                mask(a.clk_mask_shift),
                mask(a.clk_en_shift),
                mask(a.clk_y_shift),
                mask(a.clk_a_shift),
            )),
        ));
        s.push_str(&format!(
            "\n  {}   data regs 0x{:04X}/0x{:04X}/0x{:04X}/0x{:04X} (mask/en/y/a, bits {}/{}/{}/{})",
            pal.value("  "),
            a.data_mask_reg,
            a.data_en_reg,
            a.data_y_reg,
            a.data_a_reg,
            mask(a.data_mask_shift),
            mask(a.data_en_shift),
            mask(a.data_y_shift),
            mask(a.data_a_shift),
        ));
    }
    s
}
