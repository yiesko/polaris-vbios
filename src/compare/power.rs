use super::Table;
use super::title;
use crate::render::color::Palette;
use crate::render::sections::Section;
use crate::rom::types::ParsedRom;

pub(super) fn compare_sclk(a: &ParsedRom, b: &ParsedRom, pal: &Palette, diff_only: bool) -> String {
    let (ta, tb) = (&a.powerplay.sclk_table, &b.powerplay.sclk_table);
    let mut s = title(pal, Section::Sclk.label());
    s.push('\n');
    let mut t = Table::new(pal, diff_only);
    t.header(&a.file_name, &b.file_name);
    t.row("DPM levels (count)", ta.len(), tb.len());
    let boot_a = ta.first().map(|e| e.sclk_mhz).unwrap_or(0.0);
    let boot_b = tb.first().map(|e| e.sclk_mhz).unwrap_or(0.0);
    t.row_pct("SCLK boot/idle (MHz)", boot_a, boot_b, |v| {
        format!("{v:.0}")
    });
    let boost_a = ta.last().map(|e| e.sclk_mhz).unwrap_or(0.0);
    let boost_b = tb.last().map(|e| e.sclk_mhz).unwrap_or(0.0);
    t.row_pct("SCLK max boost (MHz)", boost_a, boost_b, |v| {
        format!("{v:.0}")
    });
    let n = ta.len().max(tb.len());
    for i in 0..n {
        let va = ta.get(i).map(|e| e.sclk_mhz);
        let vb = tb.get(i).map(|e| e.sclk_mhz);
        match (va, vb) {
            (Some(va), Some(vb)) => {
                t.row_pct(&format!("  level {i}"), va, vb, |v| format!("{v:.0} MHz"));
            }
            _ => {
                t.row(
                    &format!("  level {i}"),
                    va.map(|v| format!("{v:.0} MHz"))
                        .unwrap_or_else(|| "-".into()),
                    vb.map(|v| format!("{v:.0} MHz"))
                        .unwrap_or_else(|| "-".into()),
                );
            }
        }
    }
    s.push_str(&t.finish("(nothing differs in this section)"));
    s
}

pub(super) fn compare_mclk(a: &ParsedRom, b: &ParsedRom, pal: &Palette, diff_only: bool) -> String {
    let (ta, tb) = (&a.powerplay.mclk_table, &b.powerplay.mclk_table);
    let mut s = title(pal, Section::Mclk.label());
    s.push('\n');
    let mut t = Table::new(pal, diff_only);
    t.header(&a.file_name, &b.file_name);
    t.row("DPM levels (count)", ta.len(), tb.len());
    let n = ta.len().max(tb.len());
    for i in 0..n {
        let va = ta.get(i).map(|e| e.mclk_mhz);
        let vb = tb.get(i).map(|e| e.mclk_mhz);
        match (va, vb) {
            (Some(va), Some(vb)) => {
                t.row_pct(&format!("level {i}"), va, vb, |v| format!("{v:.0} MHz"));
            }
            _ => {
                t.row(
                    &format!("level {i}"),
                    va.map(|v| format!("{v:.0} MHz"))
                        .unwrap_or_else(|| "-".into()),
                    vb.map(|v| format!("{v:.0} MHz"))
                        .unwrap_or_else(|| "-".into()),
                );
            }
        }
    }
    s.push_str(&t.finish("(nothing differs in this section)"));
    s
}

pub(super) fn compare_voltages(
    a: &ParsedRom,
    b: &ParsedRom,
    pal: &Palette,
    diff_only: bool,
) -> String {
    let mut s = title(pal, Section::Voltages.label());
    s.push('\n');
    let mut t = Table::new(pal, diff_only);
    t.header(&a.file_name, &b.file_name);
    for (name, la, lb) in [
        ("VDDC", &a.powerplay.vddc_lut, &b.powerplay.vddc_lut),
        ("VDDGFX", &a.powerplay.vddgfx_lut, &b.powerplay.vddgfx_lut),
    ] {
        let va: Vec<String> = la
            .iter()
            .filter(|e| e.valid)
            .map(|e| format!("{}mV", e.vdd_mv))
            .collect();
        let vb: Vec<String> = lb
            .iter()
            .filter(|e| e.valid)
            .map(|e| format!("{}mV", e.vdd_mv))
            .collect();
        t.row(&format!("{name} valid slots"), va.join(","), vb.join(","));
    }
    s.push_str(&t.finish("(nothing differs in this section)"));
    s
}

pub(super) fn compare_vrm(a: &ParsedRom, b: &ParsedRom, pal: &Palette, diff_only: bool) -> String {
    let mut s = title(pal, Section::Vrm.label());
    s.push('\n');
    let mut t = Table::new(pal, diff_only);
    t.header(&a.file_name, &b.file_name);
    t.row(
        "Voltage objects (count)",
        a.vrm.objects.len(),
        b.vrm.objects.len(),
    );
    let n = a.vrm.objects.len().max(b.vrm.objects.len());
    for i in 0..n {
        let oa = a.vrm.objects.get(i);
        let ob = b.vrm.objects.get(i);
        let fmt = |o: &crate::rom::types::VoltageObject| {
            format!("{} via {}", o.voltage_type_name, o.mode_name)
        };
        t.row(
            &format!("object {i}"),
            oa.map(fmt).unwrap_or_else(|| "-".into()),
            ob.map(fmt).unwrap_or_else(|| "-".into()),
        );
        if let (Some(oa), Some(ob)) = (oa, ob) {
            use crate::rom::types::VoltageObjectDetail as D;
            if let (
                D::I2cInitSeq {
                    regulator_name: ra, ..
                },
                D::I2cInitSeq {
                    regulator_name: rb, ..
                },
            ) = (&oa.detail, &ob.detail)
            {
                t.row(
                    &format!("  regulator (obj. {i})"),
                    ra.clone().unwrap_or_else(|| "uncatalogued".into()),
                    rb.clone().unwrap_or_else(|| "uncatalogued".into()),
                );
            }
        }
    }
    s.push_str(&t.finish("(nothing differs in this section)"));
    s
}

pub(super) fn compare_mm(a: &ParsedRom, b: &ParsedRom, pal: &Palette, diff_only: bool) -> String {
    let mut s = title(pal, Section::Multimedia.label());
    s.push('\n');
    let mut t = Table::new(pal, diff_only);
    t.header(&a.file_name, &b.file_name);
    t.row(
        "Levels (count)",
        a.powerplay.mm_table.len(),
        b.powerplay.mm_table.len(),
    );
    t.row(
        "VCE states (count)",
        a.powerplay.vce_states.len(),
        b.powerplay.vce_states.len(),
    );
    s.push_str(&t.finish("(nothing differs in this section)"));
    s
}

pub(super) fn compare_powertune(
    a: &ParsedRom,
    b: &ParsedRom,
    pal: &Palette,
    diff_only: bool,
) -> String {
    let mut s = title(pal, Section::Powertune.label());
    s.push('\n');
    let mut t = Table::new(pal, diff_only);
    t.header(&a.file_name, &b.file_name);
    match (&a.powerplay.powertune, &b.powerplay.powertune) {
        (Some(pa), Some(pb)) => {
            t.row_pct("TDP (W)", pa.tdp_w as f64, pb.tdp_w as f64, |v| {
                format!("{v:.0}")
            });
            t.row_pct("TDC (A)", pa.tdc_a as f64, pb.tdc_a as f64, |v| {
                format!("{v:.0}")
            });
            t.row_pct(
                "Max power delivery (W)",
                pa.max_power_delivery_limit_w as f64,
                pb.max_power_delivery_limit_w as f64,
                |v| format!("{v:.0}"),
            );
            t.row_pct(
                "TjMax edge (°C)",
                pa.tjmax_c as f64,
                pb.tjmax_c as f64,
                |v| format!("{v:.0}"),
            );
            t.row_pct(
                "Hotspot limit (°C)",
                pa.temp_limit_hotspot_c as f64,
                pb.temp_limit_hotspot_c as f64,
                |v| format!("{v:.0}"),
            );
        }
        _ => {
            t.note("  (PowerTune table missing in at least one ROM)");
        }
    }
    let la = a.powerplay.hard_limits.len();
    let lb = b.powerplay.hard_limits.len();
    t.row("Hard limits (count)", la, lb);
    s.push_str(&t.finish("(nothing differs in this section)"));
    s
}

pub(super) fn compare_fan(a: &ParsedRom, b: &ParsedRom, pal: &Palette, diff_only: bool) -> String {
    let mut s = title(pal, Section::Fan.label());
    s.push('\n');
    let mut t = Table::new(pal, diff_only);
    t.header(&a.file_name, &b.file_name);
    match (&a.powerplay.fan_table, &b.powerplay.fan_table) {
        (Some(fa), Some(fb)) => {
            t.row(
                "T.min / T.med / T.high (°C)",
                format!("{:.0}/{:.0}/{:.0}", fa.t_min_c, fa.t_med_c, fa.t_high_c),
                format!("{:.0}/{:.0}/{:.0}", fb.t_min_c, fb.t_med_c, fb.t_high_c),
            );
            t.row(
                "PWM min/med/high (%)",
                format!(
                    "{:.0}/{:.0}/{:.0}",
                    fa.pwm_min_pct, fa.pwm_med_pct, fa.pwm_high_pct
                ),
                format!(
                    "{:.0}/{:.0}/{:.0}",
                    fb.pwm_min_pct, fb.pwm_med_pct, fb.pwm_high_pct
                ),
            );
            t.row_pct(
                "Max RPM",
                fa.fan_rpm_max as f64,
                fb.fan_rpm_max as f64,
                |v| format!("{v:.0}"),
            );
            t.row("Zero RPM", fa.zero_rpm_enable != 0, fb.zero_rpm_enable != 0);
        }
        _ => {
            t.note("  (fan table missing in at least one ROM)");
        }
    }
    s.push_str(&t.finish("(nothing differs in this section)"));
    s
}

pub(super) fn compare_pcie(a: &ParsedRom, b: &ParsedRom, pal: &Palette, diff_only: bool) -> String {
    let mut s = title(pal, Section::Pcie.label());
    s.push('\n');
    let mut t = Table::new(pal, diff_only);
    t.header(&a.file_name, &b.file_name);
    t.row(
        "PCIe levels (count)",
        a.powerplay.pcie_table.len(),
        b.powerplay.pcie_table.len(),
    );
    s.push_str(&t.finish("(nothing differs in this section)"));
    s
}
