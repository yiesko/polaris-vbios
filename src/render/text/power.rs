use super::{heading, kv};
use crate::render::color::Palette;
use crate::render::color::pad;
use crate::render::sections::Section;
use crate::rom::types::ParsedRom;

pub(super) fn render_sclk(rom: &ParsedRom, pal: &Palette) -> String {
    let mut s = heading(pal, Section::Sclk.label());
    s.push('\n');
    s.push_str(&format!(
        "  {} {} {} {}\n",
        pal.label(&pad("Level", 6)),
        pal.label(&pad("SCLK (MHz)", 12)),
        pal.label(&pad("VDD index (leakage)", 20)),
        pal.label("VDDC offset (mV)")
    ));
    for e in &rom.powerplay.sclk_table {
        s.push_str(&format!(
            "  {} {} {} {}\n",
            pad(&e.level.to_string(), 6),
            pal.value(&pad(&format!("{:.0}", e.sclk_mhz), 12)),
            pad(&e.vdd_index.to_string(), 20),
            e.vddc_offset_mv
        ));
    }
    if !rom.powerplay.states.is_empty() {
        s.push_str("\n  PowerPlay states (engine/memory DPM indices and classification):\n");
        s.push_str(&format!(
            "  {} {} {} {}\n",
            pal.label(&pad("State", 7)),
            pal.label(&pad("SCLK idx", 9)),
            pal.label(&pad("MCLK idx", 9)),
            pal.label("Classification")
        ));
        for (i, st) in rom.powerplay.states.iter().enumerate() {
            let cls = if st.classification_decoded.is_empty() {
                "none".to_string()
            } else {
                st.classification_decoded.join(", ")
            };
            s.push_str(&format!(
                "  {} {} {} {}\n",
                pad(&i.to_string(), 7),
                pad(&st.engine_clock_index.to_string(), 9),
                pad(&st.memory_clock_index.to_string(), 9),
                pal.value(&cls)
            ));
        }
    }
    s.push_str(&format!(
        "\n{}",
        pal.warn(
            "Note: on Polaris the voltage for each SCLK level is computed at runtime by the SMU\n\
             via AVFS (leakage-based), not a fixed value stored in the ROM. The 'VDD index' is the\n\
             leakage bin identifier, not a direct index into the voltage table."
        )
    ));
    s
}

pub(super) fn render_mclk(rom: &ParsedRom, pal: &Palette) -> String {
    let mut s = heading(pal, Section::Mclk.label());
    s.push('\n');
    s.push_str(&format!(
        "  {} {} {} {} {}\n",
        pal.label(&pad("Level", 6)),
        pal.label(&pad("MCLK (MHz)", 12)),
        pal.label(&pad("VDDC (mV)", 12)),
        pal.label(&pad("VDDCI (mV)", 10)),
        pal.label("MVDD (mV)")
    ));
    for e in &rom.powerplay.mclk_table {
        let vddc = e
            .vddc_resolved_mv
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".to_string());
        s.push_str(&format!(
            "  {} {} {} {} {}\n",
            pad(&e.level.to_string(), 6),
            pal.value(&pad(&format!("{:.0}", e.mclk_mhz), 12)),
            pad(&vddc, 12),
            pad(&e.vddci_mv.to_string(), 10),
            e.mvdd_mv
        ));
    }
    s
}

pub(super) fn render_voltages(rom: &ParsedRom, pal: &Palette) -> String {
    let mut s = heading(pal, Section::Voltages.label());
    for (name, lut) in [
        ("VDDC", &rom.powerplay.vddc_lut),
        ("VDDGFX", &rom.powerplay.vddgfx_lut),
    ] {
        if lut.is_empty() {
            continue;
        }
        s.push('\n');
        s.push_str(&pal.label(&format!("{name} - valid slots: ")));
        let valid: Vec<String> = lut
            .iter()
            .filter(|e| e.valid)
            .map(|e| format!("idx {} = {} mV", e.index, e.vdd_mv))
            .collect();
        s.push_str(&valid.join(", "));
        let n_invalid = lut.iter().filter(|e| !e.valid).count();
        if n_invalid > 0 {
            s.push('\n');
            s.push_str(&pal.label(&format!(
                "  ({n_invalid} reserved/placeholder slot(s) from the ROM itself, ignored)"
            )));
        }
        s.push('\n');
    }
    s
}

pub(super) fn render_vrm(rom: &ParsedRom, pal: &Palette) -> String {
    let vrm = &rom.vrm;
    let mut s = heading(pal, Section::Vrm.label());
    if !vrm.recognized_format {
        s.push_str(&format!(
            "\n  {} unrecognized VoltageObjectInfo format (revision {}.{}, expected 3.1) \
             - skipped detailed parsing to avoid misinterpreting data.",
            pal.warn("⚠"),
            vrm.fmt_rev,
            vrm.cont_rev
        ));
        return s;
    }
    if vrm.objects.is_empty() {
        s.push_str("\n  (no voltage objects found)");
        return s;
    }
    s.push('\n');
    for obj in &vrm.objects {
        s.push_str(&format!(
            "  {} controlled via {}\n",
            pal.value(&obj.voltage_type_name),
            obj.mode_name
        ));
        use crate::rom::types::VoltageObjectDetail as D;
        match &obj.detail {
            D::GpioLut {
                gpio_cntl_id,
                phase_delay_us,
                lut_mv,
                ..
            } => {
                s.push_str(&format!(
                    "    GPIO cntl id {gpio_cntl_id} · phase delay {phase_delay_us}µs · {} voltage step(s): {}\n",
                    lut_mv.len(),
                    lut_mv.iter().map(|v| format!("{v}mV")).collect::<Vec<_>>().join(", ")
                ));
            }
            D::I2cInitSeq {
                regulator_id,
                regulator_name,
                i2c_line,
                i2c_address,
                init_pairs,
            } => {
                let name = regulator_name.as_deref().unwrap_or("uncatalogued");
                s.push_str(&format!(
                    "    regulator: {} (id 0x{regulator_id:02X}) · i2c line {i2c_line} address 0x{i2c_address:02X}\n",
                    pal.good(name)
                ));
                if !init_pairs.is_empty() {
                    s.push_str(&format!(
                        "    {} init pair(s) (code→raw value, {}):\n",
                        init_pairs.len(),
                        pal.label("not confirmed to be voltage in mV - see note below")
                    ));
                    for (code, val) in init_pairs {
                        s.push_str(&format!("      0x{code:04X} → {val}\n"));
                    }
                }
            }
            D::Svid2 {
                svd_gpio_id,
                svc_gpio_id,
                load_line_psi_raw,
            } => {
                s.push_str(&format!(
                    "    GPIO SVD {svd_gpio_id} / SVC {svc_gpio_id} · load-line/PSI (raw) 0x{load_line_psi_raw:04X}\n"
                ));
            }
            D::Evv { entries } => {
                let real: Vec<_> = entries.iter().filter(|e| e.dpm_sclk_mhz > 0.0).collect();
                s.push_str(&format!(
                    "    {} EVV level(s) (SCLK → leakage-based voltage adjust):\n",
                    real.len()
                ));
                for e in real {
                    s.push_str(&format!(
                        "      {:.0} MHz → offset {:+} mV (table index {}, state {})\n",
                        e.dpm_sclk_mhz, e.v_adj_offset_mv, e.dpm_v_index, e.dpm_state
                    ));
                }
            }
            D::LeakageLut { entries_count } => {
                s.push_str(&format!(
                    "    leakage table with {entries_count} entry(ies) (not detailed)\n"
                ));
            }
            D::Unknown => {
                s.push_str(&format!(
                    "    {} unrecognized object mode (0x{:02X})\n",
                    pal.warn("⚠"),
                    obj.mode_raw
                ));
            }
        }
    }
    s.push('\n');
    s.push_str(&pal.label(
        "Note: voltage regulator names (when shown) come from the official \
         atombios.h comments in the amdgpu driver - not reverse engineering. \
         The I2C \"init pairs\" are not always voltages in mV (the header names \
         the field that way, but real data sometimes shows register writes to \
         the regulator) - hence they appear as raw values without assuming a unit.",
    ));
    s
}

pub(super) fn render_mm(rom: &ParsedRom, pal: &Palette) -> String {
    let mut s = heading(pal, Section::Multimedia.label());
    if rom.powerplay.mm_table.is_empty() {
        s.push_str("\n  (table not present in this ROM)");
        return s;
    }
    s.push('\n');
    s.push_str(&format!(
        "  {} {} {} {} {}\n",
        pal.label(&pad("Level", 6)),
        pal.label(&pad("UVD dclk", 10)),
        pal.label(&pad("UVD vclk", 10)),
        pal.label(&pad("VCE eclk", 10)),
        pal.label("SAMU clk")
    ));
    for (i, e) in rom.powerplay.mm_table.iter().enumerate() {
        s.push_str(&format!(
            "  {} {} {} {} {:.0} MHz\n",
            pad(&i.to_string(), 6),
            pad(&format!("{:.0} MHz", e.uvd_dclk_mhz), 10),
            pad(&format!("{:.0} MHz", e.uvd_vclk_mhz), 10),
            pad(&format!("{:.0} MHz", e.vce_eclk_mhz), 10),
            e.samu_clk_mhz,
        ));
    }
    if !rom.powerplay.vce_states.is_empty() {
        s.push('\n');
        s.push_str(&pal.label("VCE states (profile → DPM level used):\n"));
        s.push_str(&format!(
            "  {} {} {} {}\n",
            pal.label(&pad("State", 8)),
            pal.label(&pad("VCE clk idx", 12)),
            pal.label(&pad("flag", 6)),
            pal.label("SCLK idx / MCLK idx")
        ));
        for v in &rom.powerplay.vce_states {
            s.push_str(&format!(
                "  {} {} {} {} / {}\n",
                pad(&v.index.to_string(), 8),
                pad(&v.vce_clock_index.to_string(), 12),
                pad(&v.flag.to_string(), 6),
                v.sclk_index,
                v.mclk_index,
            ));
        }
    }
    s
}

pub(super) fn render_powertune(rom: &ParsedRom, pal: &Palette) -> String {
    let mut s = heading(pal, Section::Powertune.label());
    match &rom.powerplay.powertune {
        None => s.push_str("\n  (table not present in this ROM)"),
        Some(pt) => {
            s.push('\n');
            s.push_str(&kv(pal, "TDP", format!("{} W", pt.tdp_w)));
            s.push('\n');
            s.push_str(&kv(pal, "TDC (current limit)", format!("{} A", pt.tdc_a)));
            s.push('\n');
            s.push_str(&kv(
                pal,
                "Max power delivery limit",
                format!("{} W", pt.max_power_delivery_limit_w),
            ));
            s.push('\n');
            s.push_str(&kv(
                pal,
                "Battery / small power limit",
                format!(
                    "{} / {} W",
                    pt.battery_power_limit_w, pt.small_power_limit_w
                ),
            ));
            s.push('\n');
            s.push_str(&kv(pal, "TjMax (edge)", format!("{} °C", pt.tjmax_c)));
            s.push('\n');
            s.push_str(&kv(
                pal,
                "Hotspot limit",
                format!("{} °C", pt.temp_limit_hotspot_c),
            ));
            s.push('\n');
            s.push_str(&kv(
                pal,
                "Software shutdown",
                format!("{} °C", pt.software_shutdown_temp_c),
            ));
            s.push('\n');
            s.push_str(&kv(
                pal,
                "VR VDDC / VR MVDD limit",
                format!(
                    "{} / {} °C",
                    pt.temp_limit_vr_vddc_c, pt.temp_limit_vr_mvdd_c
                ),
            ));
            if pt.temp_limit_plx_c > 0 && pt.temp_limit_plx_c < 200 {
                s.push('\n');
                s.push_str(&kv(pal, "PLX limit", format!("{} °C", pt.temp_limit_plx_c)));
            }
        }
    }
    if !rom.powerplay.hard_limits.is_empty() {
        s.push('\n');
        s.push_str(
            &pal.label("\nHard Limit table (absolute limits, not overridable by overdrive):\n"),
        );
        for (i, h) in rom.powerplay.hard_limits.iter().enumerate() {
            s.push_str(&format!(
                "  level {i}: SCLK ≤ {:.0} MHz · MCLK ≤ {:.0} MHz · VDDC ≤ {} mV · VDDCI ≤ {} mV · VDDGFX ≤ {} mV\n",
                h.sclk_limit_mhz, h.mclk_limit_mhz, h.vddc_limit_mv, h.vddci_limit_mv, h.vddgfx_limit_mv
            ));
        }
    }
    s
}

pub(super) fn render_fan(rom: &ParsedRom, pal: &Palette) -> String {
    let mut s = heading(pal, Section::Fan.label());
    match &rom.powerplay.fan_table {
        None => s.push_str("\n  (table not present in this ROM)"),
        Some(ft) => {
            s.push('\n');
            s.push_str(&kv(
                pal,
                "T.min / T.med / T.high",
                format!(
                    "{:.0}°C / {:.0}°C / {:.0}°C",
                    ft.t_min_c, ft.t_med_c, ft.t_high_c
                ),
            ));
            s.push('\n');
            s.push_str(&kv(
                pal,
                "PWM min / med / high",
                format!(
                    "{:.0}% / {:.0}% / {:.0}%",
                    ft.pwm_min_pct, ft.pwm_med_pct, ft.pwm_high_pct
                ),
            ));
            s.push('\n');
            s.push_str(&kv(
                pal,
                "T.max (shutdown)",
                format!("{:.0} °C", ft.t_max_c),
            ));
            s.push('\n');
            s.push_str(&kv(pal, "Max RPM", ft.fan_rpm_max));
            s.push('\n');
            s.push_str(&kv(
                pal,
                "Zero RPM",
                if ft.zero_rpm_enable != 0 {
                    format!(
                        "enabled (stops at {}°C, restarts at {}°C)",
                        ft.fan_stop_temperature_c, ft.fan_start_temperature_c
                    )
                } else {
                    "disabled".to_string()
                },
            ));
            s.push('\n');
            s.push_str(&kv(
                pal,
                "Target temperature (control)",
                format!("{} °C", ft.target_temperature_c),
            ));
        }
    }
    s
}

pub(super) fn render_pcie(rom: &ParsedRom, pal: &Palette) -> String {
    let mut s = heading(pal, Section::Pcie.label());
    if rom.powerplay.pcie_table.is_empty() {
        s.push_str("\n  (table not present in this ROM)");
        return s;
    }
    s.push('\n');
    for (i, e) in rom.powerplay.pcie_table.iter().enumerate() {
        s.push_str(&format!(
            "  level {i}: gen index {}, bus width {} lane(s)\n",
            e.pcie_gen, e.pcie_lane_width
        ));
    }
    if let Some(idx) = rom.powerplay.vrhot_sclk_dpm_index {
        s.push_str(&kv(pal, "VRHot forces DPM SCLK to level", idx));
    }
    s
}
