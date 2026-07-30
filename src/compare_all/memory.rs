use super::Matrix;
use super::title;
use crate::compare_util;
use crate::render::color::Palette;
use crate::render::sections::Section;
use crate::rom::types::ParsedRom;

pub(super) fn vram_section(
    roms: &[ParsedRom],
    names: &[String],
    pal: &Palette,
    diff_only: bool,
) -> String {
    let mut s = title(pal, Section::Vram.label());
    s.push('\n');
    let mut m = Matrix::new(pal, roms.len(), diff_only);
    m.header(names);
    m.row(
        "Modules (count)",
        &roms
            .iter()
            .map(|r| r.vram.num_modules.to_string())
            .collect::<Vec<_>>(),
    );
    m.row(
        "Part numbers",
        &roms
            .iter()
            .map(|r| {
                r.vram
                    .modules
                    .iter()
                    .filter(|m| !m.part_number.is_empty())
                    .map(|m| m.part_number.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .collect::<Vec<_>>(),
    );
    s.push_str(&m.finish("(nothing differs in this section)"));
    s
}

pub(super) fn straps_section(
    roms: &[ParsedRom],
    names: &[String],
    pal: &Palette,
    diff_only: bool,
) -> String {
    let mut s = title(pal, Section::Straps.label());
    s.push('\n');
    let mut m = Matrix::new(pal, roms.len(), diff_only);
    m.header(names);
    m.row(
        "Straps (count)",
        &roms
            .iter()
            .map(|r| r.vram.straps.len().to_string())
            .collect::<Vec<_>>(),
    );
    m.row(
        "Max strap (MHz)",
        &roms
            .iter()
            .map(|r| {
                format!(
                    "{:.0}",
                    r.vram
                        .straps
                        .iter()
                        .map(|s| s.clock_mhz)
                        .fold(0.0, f64::max)
                )
            })
            .collect::<Vec<_>>(),
    );

    // Match by clock, now across N ROMs - for each clock that appears
    // in ANY of the ROMs, show whether the registers match across all
    // ROMs that have that clock.
    use std::collections::BTreeSet;
    let key = |mhz: f64| mhz.round() as i64;
    let mut all_clocks: BTreeSet<i64> = BTreeSet::new();
    for r in roms {
        for strap in &r.vram.straps {
            all_clocks.insert(key(strap.clock_mhz));
        }
    }
    m.note(&pal.label("\n  Registers per clock (matched by clock value, all ROMs):"));
    for clk in all_clocks {
        let per_rom: Vec<Option<&crate::rom::types::MemoryStrap>> = roms
            .iter()
            .map(|r| r.vram.straps.iter().find(|s| key(s.clock_mhz) == clk))
            .collect();
        let present: Vec<&crate::rom::types::MemoryStrap> =
            per_rom.iter().filter_map(|x| *x).collect();
        let all_identical = present.windows(2).all(|w| w[0].values == w[1].values);
        let all_present = per_rom.iter().all(|x| x.is_some());
        if diff_only && all_identical && all_present {
            continue;
        }
        let marker = if all_identical && all_present {
            pal.good("=")
        } else {
            pal.warn("≠")
        };
        m.note(&format!("\n  {marker} {clk} MHz"));
        for (name, strap) in names.iter().zip(per_rom.iter()) {
            match strap {
                Some(s) => m.note(&format!(
                    "    {name}: {}",
                    compare_util::fmt_vals(&s.values)
                )),
                None => m.note(&format!("    {name}: - (absent)")),
            };
        }
    }
    s.push_str(&m.finish("(nothing differs in this section)"));
    s
}

pub(super) fn caps_section(
    roms: &[ParsedRom],
    names: &[String],
    pal: &Palette,
    diff_only: bool,
) -> String {
    let mut s = title(pal, Section::Caps.label());
    s.push('\n');
    let mut m = Matrix::new(pal, roms.len(), diff_only);
    m.header(names);
    m.row(
        "platform_caps (hex)",
        &roms
            .iter()
            .map(|r| format!("0x{:X}", r.powerplay.platform_caps))
            .collect::<Vec<_>>(),
    );
    s.push_str(&m.finish("(nothing differs in this section)"));
    s
}
