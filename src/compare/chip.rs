use super::Table;
use super::title;
use crate::render::color::Palette;
use crate::render::sections::Section;
use crate::rom::types::ParsedRom;
use std::collections::BTreeSet;

pub(super) fn compare_asic(a: &ParsedRom, b: &ParsedRom, pal: &Palette, diff_only: bool) -> String {
    let mut s = title(pal, Section::Asic.label());
    s.push('\n');
    let mut t = Table::new(pal, diff_only);
    t.header(&a.file_name, &b.file_name);
    match (&a.asic, &b.asic) {
        (Some(x), Some(y)) => {
            t.row(
                "GFX IP version",
                format!("{}.{}", x.gfx_ip_maj_ver, x.gfx_ip_min_ver),
                format!("{}.{}", y.gfx_ip_maj_ver, y.gfx_ip_min_ver),
            );
            t.row(
                "Shader engines (SE)",
                x.max_shader_engines,
                y.max_shader_engines,
            );
            t.row("Shader arrays per SE", x.max_sh_per_se, y.max_sh_per_se);
            t.row("Compute units per SH", x.max_cu_per_sh, y.max_cu_per_sh);
            t.row(
                "Total CUs (max)",
                x.max_shader_engines as u32 * x.max_sh_per_se as u32 * x.max_cu_per_sh as u32,
                y.max_shader_engines as u32 * y.max_sh_per_se as u32 * y.max_cu_per_sh as u32,
            );
            t.row(
                "Render backends per SE",
                x.max_backends_per_se,
                y.max_backends_per_se,
            );
            t.row("Tile pipes", x.max_tile_pipes, y.max_tile_pipes);
            t.row(
                "Texture channel caches",
                x.max_texture_channel_caches,
                y.max_texture_channel_caches,
            );
        }
        (Some(_), None) => {
            t.note(&format!("  GFX_Info table present only in {}", a.file_name));
        }
        (None, Some(_)) => {
            t.note(&format!("  GFX_Info table present only in {}", b.file_name));
        }
        (None, None) => {
            t.note("  GFX_Info table not present in either ROM");
        }
    }
    s.push_str(&t.finish("(nothing differs in this section)"));
    s
}

pub(super) fn compare_smu(a: &ParsedRom, b: &ParsedRom, pal: &Palette, diff_only: bool) -> String {
    let mut s = title(pal, Section::Smu.label());
    s.push('\n');
    let mut t = Table::new(pal, diff_only);
    t.header(&a.file_name, &b.file_name);
    match (&a.smu, &b.smu) {
        (Some(x), Some(y)) => {
            t.row("SMU firmware version", x.smu_ver, y.smu_ver);
            t.row(
                "Share power source",
                x.share_power_source,
                y.share_power_source,
            );
            t.row(
                "FCW ranges (count)",
                x.fcw_entries.len(),
                y.fcw_entries.len(),
            );
            let n = x.fcw_entries.len().max(y.fcw_entries.len());
            for i in 0..n {
                let e = (x.fcw_entries.get(i), y.fcw_entries.get(i));
                match e {
                    (Some(a1), Some(b1)) => {
                        t.row_pct(
                            &format!("FCW range {i} max SCLK"),
                            a1.max_sclk_mhz,
                            b1.max_sclk_mhz,
                            |v| format!("{v:.0} MHz"),
                        );
                    }
                    (Some(_), None) => {
                        t.note(&format!("  FCW range {i} present only in {}", a.file_name));
                    }
                    (None, Some(_)) => {
                        t.note(&format!("  FCW range {i} present only in {}", b.file_name));
                    }
                    // invariant: i < max(x.len(), y.len()) guarantees Some(_) in at
                    // least one of the two, so (None, None) is unreachable
                    (None, None) => unreachable!(),
                }
            }
        }
        (Some(_), None) => {
            t.note(&format!("  SMU_Info table present only in {}", a.file_name));
        }
        (None, Some(_)) => {
            t.note(&format!("  SMU_Info table present only in {}", b.file_name));
        }
        (None, None) => {
            t.note("  SMU_Info table not present in either ROM");
        }
    }
    s.push_str(&t.finish("(nothing differs in this section)"));
    s
}

pub(super) fn compare_power(
    a: &ParsedRom,
    b: &ParsedRom,
    pal: &Palette,
    diff_only: bool,
) -> String {
    let mut s = title(pal, Section::Power.label());
    s.push('\n');
    let mut t = Table::new(pal, diff_only);
    t.header(&a.file_name, &b.file_name);
    match (&a.power_source, &b.power_source) {
        (Some(x), Some(y)) => {
            t.row("Power sources (count)", x.objects.len(), y.objects.len());
            let n = x.objects.len().max(y.objects.len());
            for i in 0..n {
                match (x.objects.get(i), y.objects.get(i)) {
                    (Some(oa), Some(ob)) => {
                        t.row(
                            &format!("source {i} (type)"),
                            &oa.source_name,
                            &ob.source_name,
                        );
                        t.row_pct(
                            &format!("source {i} sensed power"),
                            oa.sensed_power_w as f64,
                            ob.sensed_power_w as f64,
                            |v| format!("{v:.0} W"),
                        );
                    }
                    (Some(_), None) => {
                        t.note(&format!("  source {i} present only in {}", a.file_name));
                    }
                    (None, Some(_)) => {
                        t.note(&format!("  source {i} present only in {}", b.file_name));
                    }
                    // invariant: i < max(x.len(), y.len()) guarantees Some(_) in at
                    // least one of the two, so (None, None) is unreachable
                    (None, None) => unreachable!(),
                }
            }
        }
        (Some(_), None) => {
            t.note(&format!(
                "  PowerSourceInfo table present only in {}",
                a.file_name
            ));
        }
        (None, Some(_)) => {
            t.note(&format!(
                "  PowerSourceInfo table present only in {}",
                b.file_name
            ));
        }
        (None, None) => {
            t.note("  PowerSourceInfo table not present in either ROM");
        }
    }
    s.push_str(&t.finish("(nothing differs in this section)"));
    s
}

pub(super) fn compare_gpio(a: &ParsedRom, b: &ParsedRom, pal: &Palette, diff_only: bool) -> String {
    let mut s = title(pal, Section::Gpio.label());
    s.push('\n');
    let mut t = Table::new(pal, diff_only);
    t.header(&a.file_name, &b.file_name);
    match (&a.gpio_pin_lut, &b.gpio_pin_lut) {
        (Some(x), Some(y)) => {
            t.row("GPIO pins (count)", x.pins.len(), y.pins.len());
            let ids_a: BTreeSet<_> = x.pins.iter().map(|p| p.gpio_id).collect();
            let ids_b: BTreeSet<_> = y.pins.iter().map(|p| p.gpio_id).collect();
            let only_a: Vec<_> = ids_a.difference(&ids_b).collect();
            let only_b: Vec<_> = ids_b.difference(&ids_a).collect();
            if !only_a.is_empty() {
                t.note(&format!(
                    "  {} GPIO ids only in {}: {:?}",
                    pal.warn("→"),
                    a.file_name,
                    only_a
                ));
            }
            if !only_b.is_empty() {
                t.note(&format!(
                    "  {} GPIO ids only in {}: {:?}",
                    pal.warn("→"),
                    b.file_name,
                    only_b
                ));
            }
            if only_a.is_empty() && only_b.is_empty() && !diff_only {
                t.note(&format!("  {}", pal.good("same GPIO ids in both ROMs")));
            }
        }
        (Some(_), None) => {
            t.note(&format!(
                "  GPIO_Pin_LUT table present only in {}",
                a.file_name
            ));
        }
        (None, Some(_)) => {
            t.note(&format!(
                "  GPIO_Pin_LUT table present only in {}",
                b.file_name
            ));
        }
        (None, None) => {
            t.note("  GPIO_Pin_LUT table not present in either ROM");
        }
    }
    s.push_str(&t.finish("(nothing differs in this section)"));
    s
}

pub(super) fn compare_profiling(
    a: &ParsedRom,
    b: &ParsedRom,
    pal: &Palette,
    diff_only: bool,
) -> String {
    let mut s = title(pal, Section::Profiling.label());
    s.push('\n');
    let mut t = Table::new(pal, diff_only);
    t.header(&a.file_name, &b.file_name);
    match (&a.profiling, &b.profiling) {
        (Some(x), Some(y)) => {
            t.row_pct(
                "Max VDDC (mV)",
                x.max_vddc_mv as f64,
                y.max_vddc_mv as f64,
                |v| format!("{:.0}", v / 100.0),
            );
            t.row_pct(
                "Min VDDC (mV)",
                x.min_vddc_mv as f64,
                y.min_vddc_mv as f64,
                |v| format!("{:.0}", v / 100.0),
            );
            t.row(
                "Max voltage (0.25mV units)",
                x.max_voltage_0_25mv,
                y.max_voltage_0_25mv,
            );
            t.row("Load line slope", x.load_line_slop, y.load_line_slop);
            t.row(
                "Leakage efuse (idx/bit/len)",
                format!(
                    "{}/{}/{}",
                    x.lkg_euse_index, x.lkg_efuse_bit_lsb, x.lkg_efuse_length
                ),
                format!(
                    "{}/{}/{}",
                    y.lkg_euse_index, y.lkg_efuse_bit_lsb, y.lkg_efuse_length
                ),
            );
            t.row(
                "Leakage encode max/min",
                format!("{}/{}", x.lkg_encode_max, x.lkg_encode_min),
                format!("{}/{}", y.lkg_encode_max, y.lkg_encode_min),
            );
            t.row_pct(
                "EVV default VDDC (V)",
                x.evv_default_vddc_v100000 as f64,
                y.evv_default_vddc_v100000 as f64,
                |v| format!("{:.3}", v / 100000.0),
            );
            let n = x
                .tdc_limit_per_dpm_a10
                .len()
                .max(y.tdc_limit_per_dpm_a10.len());
            for i in 0..n {
                match (
                    x.tdc_limit_per_dpm_a10.get(i),
                    y.tdc_limit_per_dpm_a10.get(i),
                ) {
                    (Some(a1), Some(b1)) => {
                        t.row_pct(&format!("TDC DPM{i} (A)"), *a1 as f64, *b1 as f64, |v| {
                            format!("{:.0}", v / 10.0)
                        });
                    }
                    (Some(_), None) => {
                        t.note(&format!("  TDC DPM{i} present only in {}", a.file_name));
                    }
                    (None, Some(_)) => {
                        t.note(&format!("  TDC DPM{i} present only in {}", b.file_name));
                    }
                    // invariant: i < max(x.len(), y.len()) guarantees Some(_) in at
                    // least one of the two, so (None, None) is unreachable
                    (None, None) => unreachable!(),
                }
            }
        }
        (Some(_), None) => {
            t.note(&format!(
                "  ASIC_ProfilingInfo table present only in {}",
                a.file_name
            ));
        }
        (None, Some(_)) => {
            t.note(&format!(
                "  ASIC_ProfilingInfo table present only in {}",
                b.file_name
            ));
        }
        (None, None) => {
            t.note("  ASIC_ProfilingInfo table not present in either ROM");
        }
    }
    s.push_str(&t.finish("(nothing differs in this section)"));
    s
}

pub(super) fn compare_ss(a: &ParsedRom, b: &ParsedRom, pal: &Palette, diff_only: bool) -> String {
    let mut s = title(pal, Section::Ss.label());
    s.push('\n');
    let mut t = Table::new(pal, diff_only);
    t.header(&a.file_name, &b.file_name);
    match (&a.ss, &b.ss) {
        (Some(x), Some(y)) => {
            t.row("SS entries (count)", x.entries.len(), y.entries.len());
            let n = x.entries.len().max(y.entries.len());
            for i in 0..n {
                match (x.entries.get(i), y.entries.get(i)) {
                    (Some(ea), Some(eb)) => {
                        t.row(
                            &format!("entry {i} clock"),
                            crate::rom::ss::ss_clock_name(ea.clock_indication),
                            crate::rom::ss::ss_clock_name(eb.clock_indication),
                        );
                        t.row(
                            &format!("  {i} pct (mode 0x{:X})", ea.spread_mode),
                            format!("{:.2}%", ea.spread_pct),
                            format!("{:.2}%", eb.spread_pct),
                        );
                        t.row(
                            &format!("  {i} rate (kHz)"),
                            format!("{:.1}", ea.spread_rate_hz10 as f64 / 100.0),
                            format!("{:.1}", eb.spread_rate_hz10 as f64 / 100.0),
                        );
                    }
                    (Some(_), None) => {
                        t.note(&format!("  SS entry {i} present only in {}", a.file_name));
                    }
                    (None, Some(_)) => {
                        t.note(&format!("  entry {i} present only in {}", b.file_name));
                    }
                    // invariant: i < max(x.len(), y.len()) guarantees Some(_) in at
                    // least one of the two, so (None, None) is unreachable
                    (None, None) => unreachable!(),
                }
            }
        }
        (Some(_), None) => {
            t.note(&format!(
                "  ASIC_InternalSS_Info table present only in {}",
                a.file_name
            ));
        }
        (None, Some(_)) => {
            t.note(&format!(
                "  ASIC_InternalSS_Info table present only in {}",
                b.file_name
            ));
        }
        (None, None) => {
            t.note("  ASIC_InternalSS_Info table not present in either ROM");
        }
    }
    s.push_str(&t.finish("(nothing differs in this section)"));
    s
}

pub(super) fn compare_vesa(a: &ParsedRom, b: &ParsedRom, pal: &Palette, diff_only: bool) -> String {
    let mut s = title(pal, Section::Vesa.label());
    s.push('\n');
    let mut t = Table::new(pal, diff_only);
    t.header(&a.file_name, &b.file_name);
    match (&a.vesa, &b.vesa) {
        (Some(x), Some(y)) => {
            t.row("Native VESA modes (count)", x.modes.len(), y.modes.len());
            let n = x.modes.len().max(y.modes.len());
            for i in 0..n {
                match (x.modes.get(i), y.modes.get(i)) {
                    (Some(ma), Some(mb)) => {
                        t.row(
                            &format!("mode {i} resolution"),
                            format!("{}x{}", ma.h_active, ma.v_active),
                            format!("{}x{}", mb.h_active, mb.v_active),
                        );
                        t.row(
                            &format!("  {i} refresh (Hz)"),
                            format!("{:.2}", ma.refresh_rate_hz),
                            format!("{:.2}", mb.refresh_rate_hz),
                        );
                        t.row(
                            &format!("  {i} pixel clock (MHz)"),
                            format!("{:.1}", ma.pixel_clock_mhz),
                            format!("{:.1}", mb.pixel_clock_mhz),
                        );
                        t.row(
                            &format!("  {i} sync polarity"),
                            ma.sync_polarity.clone(),
                            mb.sync_polarity.clone(),
                        );
                    }
                    (Some(_), None) => {
                        t.note(&format!("  VESA mode {i} present only in {}", a.file_name));
                    }
                    (None, Some(_)) => {
                        t.note(&format!("  mode {i} present only in {}", b.file_name));
                    }
                    // invariant: i < max(x.len(), y.len()) guarantees Some(_) in at
                    // least one of the two, so (None, None) is unreachable
                    (None, None) => unreachable!(),
                }
            }
        }
        (Some(_), None) => {
            t.note(&format!(
                "  StandardVESA_Timing table present only in {}",
                a.file_name
            ));
        }
        (None, Some(_)) => {
            t.note(&format!(
                "  StandardVESA_Timing table present only in {}",
                b.file_name
            ));
        }
        (None, None) => {
            t.note("  StandardVESA_Timing table not present in either ROM");
        }
    }
    s.push_str(&t.finish("(nothing differs in this section)"));
    s
}

pub(super) fn compare_i2c(a: &ParsedRom, b: &ParsedRom, pal: &Palette, diff_only: bool) -> String {
    let mut s = title(pal, Section::I2c.label());
    s.push('\n');
    let mut t = Table::new(pal, diff_only);
    t.header(&a.file_name, &b.file_name);
    match (&a.i2c, &b.i2c) {
        (Some(x), Some(y)) => {
            t.row(
                "I2C lines (count)",
                x.assignments.len(),
                y.assignments.len(),
            );
            let ids_a: Vec<_> = x.assignments.iter().map(|a| a.i2c_id).collect();
            let ids_b: Vec<_> = y.assignments.iter().map(|a| a.i2c_id).collect();
            t.row(
                "Line ids",
                format!("{:02X?}", ids_a),
                format!("{:02X?}", ids_b),
            );
            let n = x.assignments.len().max(y.assignments.len());
            for i in 0..n {
                match (x.assignments.get(i), y.assignments.get(i)) {
                    (Some(aa), Some(ab)) => {
                        let clk = |a: &crate::rom::types::GpioI2cAssignment| {
                            format!(
                                "0x{:04X} mask/en/y/a 0x{:02X}/0x{:02X}/0x{:02X}/0x{:02X}",
                                a.clk_mask_reg,
                                a.clk_mask_shift,
                                a.clk_en_shift,
                                a.clk_y_shift,
                                a.clk_a_shift,
                            )
                        };
                        t.row(&format!("line {i} clk"), clk(aa), clk(ab));
                        t.row(
                            &format!("  {i} data"),
                            format!(
                                "0x{:04X} mask/en/y/a 0x{:02X}/0x{:02X}/0x{:02X}/0x{:02X}",
                                aa.data_mask_reg,
                                aa.data_mask_shift,
                                aa.data_en_shift,
                                aa.data_y_shift,
                                aa.data_a_shift,
                            ),
                            format!(
                                "0x{:04X} mask/en/y/a 0x{:02X}/0x{:02X}/0x{:02X}/0x{:02X}",
                                ab.data_mask_reg,
                                ab.data_mask_shift,
                                ab.data_en_shift,
                                ab.data_y_shift,
                                ab.data_a_shift,
                            ),
                        );
                    }
                    (Some(_), None) => {
                        t.note(&format!("  I2C line {i} present only in {}", a.file_name));
                    }
                    (None, Some(_)) => {
                        t.note(&format!("  line {i} present only in {}", b.file_name));
                    }
                    // invariant: i < max(x.len(), y.len()) guarantees Some(_) in at
                    // least one of the two, so (None, None) is unreachable
                    (None, None) => unreachable!(),
                }
            }
        }
        (Some(_), None) => {
            t.note(&format!(
                "  GPIO_I2C_Info table present only in {}",
                a.file_name
            ));
        }
        (None, Some(_)) => {
            t.note(&format!(
                "  GPIO_I2C_Info table present only in {}",
                b.file_name
            ));
        }
        (None, None) => {
            t.note("  GPIO_I2C_Info table not present in either ROM");
        }
    }
    s.push_str(&t.finish("(nothing differs in this section)"));
    s
}
