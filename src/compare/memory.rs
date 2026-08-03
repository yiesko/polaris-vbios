use super::Table;
use super::title;
use crate::compare_util;
use crate::render::color::Palette;
use crate::render::sections::Section;
use crate::render::text::RegNames;
use crate::rom::types::ParsedRom;
use std::collections::BTreeSet;

pub(super) fn compare_vram(a: &ParsedRom, b: &ParsedRom, pal: &Palette, diff_only: bool) -> String {
    let mut s = title(pal, Section::Vram.label());
    s.push('\n');
    let mut t = Table::new(pal, diff_only);
    t.header(&a.file_name, &b.file_name);
    t.row("Declared modules", a.vram.num_modules, b.vram.num_modules);
    let pn_a = compare_util::part_numbers(&a.vram.modules);
    let pn_b = compare_util::part_numbers(&b.vram.modules);
    t.row("Part numbers in ROM", pn_a.join(", "), pn_b.join(", "));
    // Geometry matters: --vram-size-mb / --import-vram change exactly
    // these fields, so compare them module by module.
    let per_module = |m: &[crate::rom::types::VramModule],
                      f: &dyn Fn(&crate::rom::types::VramModule) -> String| {
        m.iter()
            .map(|m| format!("{}:{}", m.index, f(m)))
            .collect::<Vec<_>>()
            .join(", ")
    };
    t.row(
        "Memory size (MB) per module",
        per_module(&a.vram.modules, &|m| m.memory_size_mb.to_string()),
        per_module(&b.vram.modules, &|m| m.memory_size_mb.to_string()),
    );
    t.row(
        "Memory vendor (raw) per module",
        per_module(&a.vram.modules, &|m| m.vendor_id_raw.to_string()),
        per_module(&b.vram.modules, &|m| m.vendor_id_raw.to_string()),
    );
    s.push_str(&t.finish("(nothing differs in this section)"));
    s
}

/// Compares memory straps matched by CLOCK (not by position in the
/// list) - two ROMs may have straps in different orders or quantities,
/// so comparing "strap 0 of A" with "strap 0 of B" does not make sense;
/// what matters is: for the same clock (e.g. 1750 MHz), are the
/// register values equal or not?
pub(super) fn compare_straps(
    a: &ParsedRom,
    b: &ParsedRom,
    pal: &Palette,
    diff_only: bool,
    reg_names: RegNames,
) -> String {
    let mut s = title(pal, Section::Straps.label());
    s.push('\n');
    let mut t = Table::new(pal, diff_only);
    t.header(&a.file_name, &b.file_name);
    t.row(
        "Available straps (count)",
        a.vram.straps.len(),
        b.vram.straps.len(),
    );
    let max_a = a
        .vram
        .straps
        .iter()
        .map(|s| s.clock_mhz)
        .fold(0.0, f64::max);
    let max_b = b
        .vram
        .straps
        .iter()
        .map(|s| s.clock_mhz)
        .fold(0.0, f64::max);
    t.row_pct("Max strap (MHz)", max_a, max_b, |v| format!("{v:.0}"));
    let blocks_a: BTreeSet<u8> = a.vram.straps.iter().map(|s| s.mem_block_id).collect();
    let blocks_b: BTreeSet<u8> = b.vram.straps.iter().map(|s| s.mem_block_id).collect();
    t.row(
        "Memory blocks (vendor)",
        format!("{blocks_a:?}"),
        format!("{blocks_b:?}"),
    );

    // Match by clock (rounded to the nearest MHz, to tolerate
    // floating-point noise without requiring exact bit equality).
    let key = |mhz: f64| mhz.round() as i64;
    let clocks_a: BTreeSet<i64> = a.vram.straps.iter().map(|s| key(s.clock_mhz)).collect();
    let clocks_b: BTreeSet<i64> = b.vram.straps.iter().map(|s| key(s.clock_mhz)).collect();
    let all_clocks: BTreeSet<i64> = clocks_a.union(&clocks_b).copied().collect();

    let clocks: Vec<i64> = all_clocks.into_iter().collect();

    // Core timing x clock matrix: rows are timing fields, columns are
    // the strap clocks present in either ROM. Cells show A/B values in
    // memory-clock cycles; differing cells are highlighted, rows with
    // no differences are hidden under --diff-only.
    t.note(&pal.label("\n  Core timings per clock (cycles, A/B; · = no strap at this clock):"));
    let cell = |field: &str, clk: i64| -> (String, bool) {
        let x = compare_util::field_at(a, clk, field);
        let y = compare_util::field_at(b, clk, field);
        let content = match (x, y) {
            (Some(xv), Some(yv)) if xv == yv => format!("{xv}="),
            (Some(xv), Some(yv)) => format!("{xv}/{yv}"),
            (Some(xv), None) => format!("{xv}/-"),
            (None, Some(yv)) => format!("-/{yv}"),
            (None, None) => "·".to_string(),
        };
        (content, x != y)
    };
    let matrix = compare_util::core_matrix(
        pal,
        crate::rom::timings::CORE_TIMINGS,
        &clocks,
        8,
        true,
        diff_only,
        cell,
    );
    if !matrix.is_empty() {
        t.note(matrix.trim_end());
    }
    t.note(&pal.label("  (= equal, ≠ differs, · = absent; values are memory-clock cycles)"));

    // Everything else per clock: the remaining decoded fields, grouped
    // by the register they live in (name + offset), plus raw hex for
    // registers with no known timing layout. Fields equal in both ROMs
    // are shown in green, differing ones in yellow, · = absent.
    let field_pairs = |rom: &ParsedRom, clk: i64| {
        let Some(strap) = rom.vram.straps.iter().find(|s| key(s.clock_mhz) == clk) else {
            return Vec::new();
        };
        compare_util::strap_other_groups(
            &strap.values,
            &rom.vram.strap_reg_indices,
            crate::rom::timings::CORE_TIMINGS,
            |idx| compare_util::reg_names_label(reg_names, idx),
        )
    };
    t.note(&pal.label(
        "\n  Other fields & raw registers per clock (green = equal, yellow = differs, · = absent):",
    ));
    for clk in &clocks {
        let ga = field_pairs(a, *clk);
        let gb = field_pairs(b, *clk);
        let (identical, lines) = compare_util::other_fields_lines(pal, &[ga, gb]);
        if diff_only && identical {
            continue;
        }
        let marker = if identical {
            pal.good("=")
        } else {
            pal.warn("≠")
        };
        t.note(&format!("  {marker} {clk} MHz"));
        if !identical {
            for line in compare_util::align_other_lines(pal, &lines) {
                t.note(&line);
            }
        }
    }

    let trips_a = crate::rom::validate::cross_checks(a);
    let trips_b = crate::rom::validate::cross_checks(b);
    if !trips_a.is_empty() || !trips_b.is_empty() {
        t.note(&pal.label("\n  Hard limit cross-check (informational):"));
        for (label, trips) in [(&a.file_name, &trips_a), (&b.file_name, &trips_b)] {
            for trip in trips {
                t.note(&format!("    {} {}: {}", pal.warn("⚠"), label, trip));
            }
        }
    }
    s.push_str(&t.finish("(nothing differs in this section)"));
    s
}

pub(super) fn compare_caps(a: &ParsedRom, b: &ParsedRom, pal: &Palette, diff_only: bool) -> String {
    let mut s = title(pal, Section::Caps.label());
    s.push('\n');
    let mut t = Table::new(pal, diff_only);
    t.header(&a.file_name, &b.file_name);
    t.row(
        "platform_caps (hex)",
        format!("0x{:X}", a.powerplay.platform_caps),
        format!("0x{:X}", b.powerplay.platform_caps),
    );
    let set_a: BTreeSet<_> = a.powerplay.platform_caps_decoded.iter().collect();
    let set_b: BTreeSet<_> = b.powerplay.platform_caps_decoded.iter().collect();
    let only_a: Vec<_> = set_a.difference(&set_b).collect();
    let only_b: Vec<_> = set_b.difference(&set_a).collect();
    if !only_a.is_empty() {
        t.note(&format!(
            "  {} only in {}: {:?}",
            pal.warn("→"),
            a.file_name,
            only_a
        ));
    }
    if !only_b.is_empty() {
        t.note(&format!(
            "  {} only in {}: {:?}",
            pal.warn("→"),
            b.file_name,
            only_b
        ));
    }
    if only_a.is_empty() && only_b.is_empty() && !diff_only {
        t.note(&format!("  {}", pal.good("same flags active in both ROMs")));
    }
    s.push_str(&t.finish("(nothing differs in this section)"));
    s
}
