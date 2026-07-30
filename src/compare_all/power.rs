use super::Matrix;
use super::title;
use crate::render::color::Palette;
use crate::render::sections::Section;
use crate::rom::types::ParsedRom;

pub(super) fn sclk_section(
    roms: &[ParsedRom],
    names: &[String],
    pal: &Palette,
    diff_only: bool,
) -> String {
    let mut s = title(pal, Section::Sclk.label());
    s.push('\n');
    let mut m = Matrix::new(pal, roms.len(), diff_only);
    m.header(names);
    m.row(
        "Levels (count)",
        &roms
            .iter()
            .map(|r| r.powerplay.sclk_table.len().to_string())
            .collect::<Vec<_>>(),
    );
    m.row(
        "Max boost (MHz)",
        &roms
            .iter()
            .map(|r| {
                r.powerplay
                    .sclk_table
                    .last()
                    .map(|e| format!("{:.0}", e.sclk_mhz))
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>(),
    );
    let max_levels = roms
        .iter()
        .map(|r| r.powerplay.sclk_table.len())
        .max()
        .unwrap_or(0);
    for i in 0..max_levels {
        m.row(
            &format!("level {i}"),
            &roms
                .iter()
                .map(|r| {
                    r.powerplay
                        .sclk_table
                        .get(i)
                        .map(|e| format!("{:.0}", e.sclk_mhz))
                        .unwrap_or_else(|| "-".into())
                })
                .collect::<Vec<_>>(),
        );
    }
    s.push_str(&m.finish("(nothing differs in this section)"));
    s
}

pub(super) fn mclk_section(
    roms: &[ParsedRom],
    names: &[String],
    pal: &Palette,
    diff_only: bool,
) -> String {
    let mut s = title(pal, Section::Mclk.label());
    s.push('\n');
    let mut m = Matrix::new(pal, roms.len(), diff_only);
    m.header(names);
    m.row(
        "Levels (count)",
        &roms
            .iter()
            .map(|r| r.powerplay.mclk_table.len().to_string())
            .collect::<Vec<_>>(),
    );
    let max_levels = roms
        .iter()
        .map(|r| r.powerplay.mclk_table.len())
        .max()
        .unwrap_or(0);
    for i in 0..max_levels {
        m.row(
            &format!("level {i}"),
            &roms
                .iter()
                .map(|r| {
                    r.powerplay
                        .mclk_table
                        .get(i)
                        .map(|e| format!("{:.0}", e.mclk_mhz))
                        .unwrap_or_else(|| "-".into())
                })
                .collect::<Vec<_>>(),
        );
    }
    s.push_str(&m.finish("(nothing differs in this section)"));
    s
}

pub(super) fn voltages_section(
    roms: &[ParsedRom],
    names: &[String],
    pal: &Palette,
    diff_only: bool,
) -> String {
    let mut s = title(pal, Section::Voltages.label());
    s.push('\n');
    let mut m = Matrix::new(pal, roms.len(), diff_only);
    m.header(names);
    m.row(
        "VDDC valid slots",
        &roms
            .iter()
            .map(|r| {
                r.powerplay
                    .vddc_lut
                    .iter()
                    .filter(|e| e.valid)
                    .map(|e| e.vdd_mv.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .collect::<Vec<_>>(),
    );
    s.push_str(&m.finish("(nothing differs in this section)"));
    s
}

pub(super) fn vrm_section(
    roms: &[ParsedRom],
    names: &[String],
    pal: &Palette,
    diff_only: bool,
) -> String {
    let mut s = title(pal, Section::Vrm.label());
    s.push('\n');
    let mut m = Matrix::new(pal, roms.len(), diff_only);
    m.header(names);
    m.row(
        "Voltage objects (count)",
        &roms
            .iter()
            .map(|r| r.vrm.objects.len().to_string())
            .collect::<Vec<_>>(),
    );
    s.push_str(&m.finish("(nothing differs in this section)"));
    s
}

pub(super) fn mm_section(
    roms: &[ParsedRom],
    names: &[String],
    pal: &Palette,
    diff_only: bool,
) -> String {
    let mut s = title(pal, Section::Multimedia.label());
    s.push('\n');
    let mut m = Matrix::new(pal, roms.len(), diff_only);
    m.header(names);
    m.row(
        "Levels (count)",
        &roms
            .iter()
            .map(|r| r.powerplay.mm_table.len().to_string())
            .collect::<Vec<_>>(),
    );
    s.push_str(&m.finish("(nothing differs in this section)"));
    s
}

pub(super) fn powertune_section(
    roms: &[ParsedRom],
    names: &[String],
    pal: &Palette,
    diff_only: bool,
) -> String {
    let mut s = title(pal, Section::Powertune.label());
    s.push('\n');
    let mut m = Matrix::new(pal, roms.len(), diff_only);
    m.header(names);
    m.row(
        "TDP (W)",
        &roms
            .iter()
            .map(|r| {
                r.powerplay
                    .powertune
                    .as_ref()
                    .map(|p| p.tdp_w.to_string())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>(),
    );
    m.row(
        "TDC (A)",
        &roms
            .iter()
            .map(|r| {
                r.powerplay
                    .powertune
                    .as_ref()
                    .map(|p| p.tdc_a.to_string())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>(),
    );
    m.row(
        "TjMax edge (°C)",
        &roms
            .iter()
            .map(|r| {
                r.powerplay
                    .powertune
                    .as_ref()
                    .map(|p| p.tjmax_c.to_string())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>(),
    );
    s.push_str(&m.finish("(nothing differs in this section)"));
    s
}

pub(super) fn fan_section(
    roms: &[ParsedRom],
    names: &[String],
    pal: &Palette,
    diff_only: bool,
) -> String {
    let mut s = title(pal, Section::Fan.label());
    s.push('\n');
    let mut m = Matrix::new(pal, roms.len(), diff_only);
    m.header(names);
    m.row(
        "Max RPM",
        &roms
            .iter()
            .map(|r| {
                r.powerplay
                    .fan_table
                    .as_ref()
                    .map(|f| f.fan_rpm_max.to_string())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>(),
    );
    s.push_str(&m.finish("(nothing differs in this section)"));
    s
}

pub(super) fn pcie_section(
    roms: &[ParsedRom],
    names: &[String],
    pal: &Palette,
    diff_only: bool,
) -> String {
    let mut s = title(pal, Section::Pcie.label());
    s.push('\n');
    let mut m = Matrix::new(pal, roms.len(), diff_only);
    m.header(names);
    m.row(
        "PCIe levels (count)",
        &roms
            .iter()
            .map(|r| r.powerplay.pcie_table.len().to_string())
            .collect::<Vec<_>>(),
    );
    s.push_str(&m.finish("(nothing differs in this section)"));
    s
}
