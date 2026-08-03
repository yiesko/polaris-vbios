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
    let per_module = |m: &[crate::rom::types::VramModule]| {
        m.iter()
            .map(|m| format!("{}:{}", m.index, m.memory_size_mb))
            .collect::<Vec<_>>()
            .join(", ")
    };
    m.row(
        "Memory size (MB) per module",
        &roms
            .iter()
            .map(|r| per_module(&r.vram.modules))
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

    // The timing fields users understand at a glance. Everything else
    // (write delays, CRC, arbiter fields, registers with no known
    // layout) is summarized per clock below the matrix.
    const CORE_TIMINGS: &[&str] = &[
        "tCL", "tRCDW", "tRCDWA", "tRCDR", "tRCDRA", "tRRD", "tRC", "tRP", "tRFC", "tFAW",
    ];

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
    let clocks: Vec<i64> = all_clocks.into_iter().collect();

    // Core timing x clock matrix: rows are timing fields, columns are
    // the strap clocks present in any ROM. Each cell shows the value of
    // every ROM in order, e.g. "10/8/10"; · = no strap at this clock.
    // Cells with any difference are highlighted, rows with no
    // differences are hidden under --diff-only.
    // cell_w: 4 chars per ROM (3 digits + separator), plus a padding
    // slack so cells never glue together.
    let cell_w = roms.len() * 4 + 1;
    m.note(&pal.label(
        "\n  Core timings per clock (cycles, values in ROM order; · = no strap at this clock):",
    ));
    let header_row = format!(
        "  {:<24} {}\n",
        "timing",
        clocks
            .iter()
            .map(|clk| format!("{clk:<cell_w$}"))
            .collect::<Vec<_>>()
            .join("")
    );
    m.note(header_row.trim_end());
    for field in CORE_TIMINGS {
        let mut cells = String::new();
        let mut row_differs = false;
        for clk in &clocks {
            let vals: Vec<Option<u32>> = roms
                .iter()
                .map(|r| compare_util::field_at(r, *clk, field))
                .collect();
            let present: Vec<u32> = vals.iter().filter_map(|x| *x).collect();
            let all_equal = present.len() == vals.len() && present.windows(2).all(|w| w[0] == w[1]);
            if !all_equal {
                row_differs = true;
            }
            let content = vals
                .iter()
                .map(|v| match v {
                    Some(x) => x.to_string(),
                    None => "·".to_string(),
                })
                .collect::<Vec<_>>()
                .join("/");
            let padded = format!("{content:<cell_w$}");
            let colored = if all_equal {
                pal.good(&padded)
            } else {
                pal.warn(&padded)
            };
            cells.push_str(&colored);
        }
        if diff_only && !row_differs {
            continue;
        }
        let label = if row_differs {
            format!("{} {:<22}", pal.warn("≠"), field)
        } else {
            format!("  {:<22}", field)
        };
        m.note(&format!("  {label} {cells}"));
    }
    m.note(&pal.label("  (= equal, ≠ differs, · = absent; values are memory-clock cycles)"));

    // Everything else per clock: the remaining decoded fields, grouped
    // by the register they live in (name + offset), plus raw hex for
    // registers with no known timing layout. Fields equal across all
    // ROMs are shown in green, differing ones in yellow, · = absent.
    let field_pairs = |rom: &ParsedRom, clk: i64| {
        let Some(strap) = rom.vram.straps.iter().find(|s| key(s.clock_mhz) == clk) else {
            return Vec::new();
        };
        compare_util::strap_other_groups(
            &strap.values,
            &rom.vram.strap_reg_indices,
            CORE_TIMINGS,
            |idx| format!("0x{idx:X}"),
        )
    };
    m.note(&pal.label(
        "\n  Other fields & raw registers per clock (green = equal, yellow = differs, · = absent):",
    ));
    for clk in &clocks {
        let groups: Vec<Vec<compare_util::RegGroup>> =
            roms.iter().map(|r| field_pairs(r, *clk)).collect();
        let (identical, lines) = compare_util::other_fields_lines(pal, &groups);
        if diff_only && identical {
            continue;
        }
        let marker = if identical {
            pal.good("=")
        } else {
            pal.warn("≠")
        };
        m.note(&format!("  {marker} {clk} MHz"));
        if !identical {
            for line in compare_util::align_other_lines(pal, &lines) {
                m.note(&line);
            }
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
