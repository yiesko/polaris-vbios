use super::Matrix;
use super::title;
use crate::render::color::Palette;
use crate::render::sections::Section;
use crate::rom::types::ParsedRom;

pub(super) fn asic_section(
    roms: &[ParsedRom],
    names: &[String],
    pal: &Palette,
    diff_only: bool,
) -> String {
    let mut s = title(pal, Section::Asic.label());
    s.push('\n');
    let mut m = Matrix::new(pal, roms.len(), diff_only);
    m.header(names);
    m.row(
        "GFX IP version",
        &roms
            .iter()
            .map(|r| match &r.asic {
                Some(a) => format!("{}.{}", a.gfx_ip_maj_ver, a.gfx_ip_min_ver),
                None => "-".to_string(),
            })
            .collect::<Vec<_>>(),
    );
    m.row(
        "Shader engines (SE)",
        &roms
            .iter()
            .map(|r| {
                r.asic
                    .as_ref()
                    .map(|a| a.max_shader_engines.to_string())
                    .unwrap_or_else(|| "-".to_string())
            })
            .collect::<Vec<_>>(),
    );
    m.row(
        "Compute units per SH",
        &roms
            .iter()
            .map(|r| {
                r.asic
                    .as_ref()
                    .map(|a| a.max_cu_per_sh.to_string())
                    .unwrap_or_else(|| "-".to_string())
            })
            .collect::<Vec<_>>(),
    );
    m.row(
        "Total CUs (max)",
        &roms
            .iter()
            .map(|r| {
                r.asic
                    .as_ref()
                    .map(|a| {
                        (a.max_shader_engines as u32
                            * a.max_sh_per_se as u32
                            * a.max_cu_per_sh as u32)
                            .to_string()
                    })
                    .unwrap_or_else(|| "-".to_string())
            })
            .collect::<Vec<_>>(),
    );
    m.row(
        "Render backends per SE",
        &roms
            .iter()
            .map(|r| {
                r.asic
                    .as_ref()
                    .map(|a| a.max_backends_per_se.to_string())
                    .unwrap_or_else(|| "-".to_string())
            })
            .collect::<Vec<_>>(),
    );
    s.push_str(&m.finish("(nothing differs in this section)"));
    s
}

pub(super) fn smu_section(
    roms: &[ParsedRom],
    names: &[String],
    pal: &Palette,
    diff_only: bool,
) -> String {
    let mut s = title(pal, Section::Smu.label());
    s.push('\n');
    let mut m = Matrix::new(pal, roms.len(), diff_only);
    m.header(names);
    m.row(
        "SMU firmware version",
        &roms
            .iter()
            .map(|r| {
                r.smu
                    .as_ref()
                    .map(|s| s.smu_ver.to_string())
                    .unwrap_or_else(|| "-".to_string())
            })
            .collect::<Vec<_>>(),
    );
    m.row(
        "FCW ranges (count)",
        &roms
            .iter()
            .map(|r| {
                r.smu
                    .as_ref()
                    .map(|s| s.fcw_entries.len().to_string())
                    .unwrap_or_else(|| "-".to_string())
            })
            .collect::<Vec<_>>(),
    );
    let max_entries = roms
        .iter()
        .filter_map(|r| r.smu.as_ref())
        .map(|s| s.fcw_entries.len())
        .max()
        .unwrap_or(0);
    for i in 0..max_entries {
        m.row(
            &format!("FCW range {i} max SCLK"),
            &roms
                .iter()
                .map(|r| {
                    r.smu
                        .as_ref()
                        .and_then(|s| s.fcw_entries.get(i))
                        .map(|e| format!("{:.0} MHz", e.max_sclk_mhz))
                        .unwrap_or_else(|| "-".to_string())
                })
                .collect::<Vec<_>>(),
        );
    }
    s.push_str(&m.finish("(nothing differs in this section)"));
    s
}

pub(super) fn power_section(
    roms: &[ParsedRom],
    names: &[String],
    pal: &Palette,
    diff_only: bool,
) -> String {
    let mut s = title(pal, Section::Power.label());
    s.push('\n');
    let mut m = Matrix::new(pal, roms.len(), diff_only);
    m.header(names);
    m.row(
        "Power sources (count)",
        &roms
            .iter()
            .map(|r| {
                r.power_source
                    .as_ref()
                    .map(|p| p.objects.len().to_string())
                    .unwrap_or_else(|| "-".to_string())
            })
            .collect::<Vec<_>>(),
    );
    let max_objs = roms
        .iter()
        .filter_map(|r| r.power_source.as_ref())
        .map(|p| p.objects.len())
        .max()
        .unwrap_or(0);
    for i in 0..max_objs {
        m.row(
            &format!("source {i}"),
            &roms
                .iter()
                .map(|r| {
                    r.power_source
                        .as_ref()
                        .and_then(|p| p.objects.get(i))
                        .map(|o| o.source_name.clone())
                        .unwrap_or_else(|| "-".to_string())
                })
                .collect::<Vec<_>>(),
        );
    }
    s.push_str(&m.finish("(nothing differs in this section)"));
    s
}

pub(super) fn gpio_section(
    roms: &[ParsedRom],
    names: &[String],
    pal: &Palette,
    diff_only: bool,
) -> String {
    let mut s = title(pal, Section::Gpio.label());
    s.push('\n');
    let mut m = Matrix::new(pal, roms.len(), diff_only);
    m.header(names);
    m.row(
        "GPIO pins (count)",
        &roms
            .iter()
            .map(|r| {
                r.gpio_pin_lut
                    .as_ref()
                    .map(|g| g.pins.len().to_string())
                    .unwrap_or_else(|| "-".to_string())
            })
            .collect::<Vec<_>>(),
    );
    s.push_str(&m.finish("(nothing differs in this section)"));
    s
}

pub(super) fn profiling_section(
    roms: &[ParsedRom],
    names: &[String],
    pal: &Palette,
    diff_only: bool,
) -> String {
    let mut s = title(pal, Section::Profiling.label());
    s.push('\n');
    let mut m = Matrix::new(pal, roms.len(), diff_only);
    m.header(names);
    m.row(
        "Max VDDC (mV)",
        &roms
            .iter()
            .map(|r| {
                r.profiling
                    .as_ref()
                    .map(|p| format!("{:.0}", p.max_vddc_mv as f64 / 100.0))
                    .unwrap_or_else(|| "-".to_string())
            })
            .collect::<Vec<_>>(),
    );
    m.row(
        "Min VDDC (mV)",
        &roms
            .iter()
            .map(|r| {
                r.profiling
                    .as_ref()
                    .map(|p| format!("{:.0}", p.min_vddc_mv as f64 / 100.0))
                    .unwrap_or_else(|| "-".to_string())
            })
            .collect::<Vec<_>>(),
    );
    m.row(
        "Load line slope",
        &roms
            .iter()
            .map(|r| {
                r.profiling
                    .as_ref()
                    .map(|p| p.load_line_slop.to_string())
                    .unwrap_or_else(|| "-".to_string())
            })
            .collect::<Vec<_>>(),
    );
    m.row(
        "Max voltage (0.25mV)",
        &roms
            .iter()
            .map(|r| {
                r.profiling
                    .as_ref()
                    .map(|p| p.max_voltage_0_25mv.to_string())
                    .unwrap_or_else(|| "-".to_string())
            })
            .collect::<Vec<_>>(),
    );
    let max_tdc = roms
        .iter()
        .filter_map(|r| r.profiling.as_ref())
        .map(|p| p.tdc_limit_per_dpm_a10.len())
        .max()
        .unwrap_or(0);
    for i in 0..max_tdc {
        m.row(
            &format!("TDC DPM{i} (A)"),
            &roms
                .iter()
                .map(|r| {
                    r.profiling
                        .as_ref()
                        .and_then(|p| p.tdc_limit_per_dpm_a10.get(i))
                        .map(|v| format!("{:.0}", v / 10))
                        .unwrap_or_else(|| "-".to_string())
                })
                .collect::<Vec<_>>(),
        );
    }
    s.push_str(&m.finish("(nothing differs in this section)"));
    s
}

pub(super) fn ss_section(
    roms: &[ParsedRom],
    names: &[String],
    pal: &Palette,
    diff_only: bool,
) -> String {
    let mut s = title(pal, Section::Ss.label());
    s.push('\n');
    let mut m = Matrix::new(pal, roms.len(), diff_only);
    m.header(names);
    m.row(
        "SS entries (count)",
        &roms
            .iter()
            .map(|r| {
                r.ss.as_ref()
                    .map(|s| s.entries.len().to_string())
                    .unwrap_or_else(|| "-".to_string())
            })
            .collect::<Vec<_>>(),
    );
    let max_entries = roms
        .iter()
        .filter_map(|r| r.ss.as_ref())
        .map(|s| s.entries.len())
        .max()
        .unwrap_or(0);
    for i in 0..max_entries {
        m.row(
            &format!("entry {i} (clock/pct/rate)"),
            &roms
                .iter()
                .map(|r| {
                    r.ss.as_ref()
                        .and_then(|s| s.entries.get(i))
                        .map(|e| {
                            format!(
                                "{}/{:.2}%/{:.1}kHz",
                                crate::rom::ss::ss_clock_name(e.clock_indication),
                                e.spread_pct,
                                e.spread_rate_hz10 as f64 / 100.0
                            )
                        })
                        .unwrap_or_else(|| "-".to_string())
                })
                .collect::<Vec<_>>(),
        );
    }
    s.push_str(&m.finish("(nothing differs in this section)"));
    s
}

pub(super) fn vesa_section(
    roms: &[ParsedRom],
    names: &[String],
    pal: &Palette,
    diff_only: bool,
) -> String {
    let mut s = title(pal, Section::Vesa.label());
    s.push('\n');
    let mut m = Matrix::new(pal, roms.len(), diff_only);
    m.header(names);
    m.row(
        "Native VESA modes (count)",
        &roms
            .iter()
            .map(|r| {
                r.vesa
                    .as_ref()
                    .map(|v| v.modes.len().to_string())
                    .unwrap_or_else(|| "-".to_string())
            })
            .collect::<Vec<_>>(),
    );
    let max_modes = roms
        .iter()
        .filter_map(|r| r.vesa.as_ref())
        .map(|v| v.modes.len())
        .max()
        .unwrap_or(0);
    for i in 0..max_modes {
        m.row(
            &format!("mode {i} (res/refresh)"),
            &roms
                .iter()
                .map(|r| {
                    r.vesa
                        .as_ref()
                        .and_then(|v| v.modes.get(i))
                        .map(|m| {
                            format!(
                                "{}x{} @ {:.2}Hz · {:.1}MHz · {}",
                                m.h_active,
                                m.v_active,
                                m.refresh_rate_hz,
                                m.pixel_clock_mhz,
                                m.sync_polarity
                            )
                        })
                        .unwrap_or_else(|| "-".to_string())
                })
                .collect::<Vec<_>>(),
        );
    }
    s.push_str(&m.finish("(nothing differs in this section)"));
    s
}

pub(super) fn i2c_section(
    roms: &[ParsedRom],
    names: &[String],
    pal: &Palette,
    diff_only: bool,
) -> String {
    let mut s = title(pal, Section::I2c.label());
    s.push('\n');
    let mut m = Matrix::new(pal, roms.len(), diff_only);
    m.header(names);
    m.row(
        "I2C lines (count)",
        &roms
            .iter()
            .map(|r| {
                r.i2c
                    .as_ref()
                    .map(|i| i.assignments.len().to_string())
                    .unwrap_or_else(|| "-".to_string())
            })
            .collect::<Vec<_>>(),
    );
    m.row(
        "Line ids (0x..)",
        &roms
            .iter()
            .map(|r| {
                r.i2c
                    .as_ref()
                    .map(|i| {
                        i.assignments
                            .iter()
                            .map(|a| format!("{:02X}", a.i2c_id))
                            .collect::<Vec<_>>()
                            .join(",")
                    })
                    .unwrap_or_else(|| "-".to_string())
            })
            .collect::<Vec<_>>(),
    );
    s.push_str(&m.finish("(nothing differs in this section)"));
    s
}
