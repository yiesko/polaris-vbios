use crate::render::color::Palette;
use crate::rom::types::ParsedRom;
use crate::rom::types::StrapSliceExt;

/// Percent delta `(b - a) / a`, formatted with sign. `a == 0` has no
/// well-defined percent variation, so we show a neutral indicator
/// instead of dividing by zero. Shared by the compare reports (row
/// deltas) and the patch plan (TDP edit delta).
pub fn pct_delta(a: f64, b: f64) -> String {
    if a == 0.0 {
        if b == 0.0 {
            "0%".to_string()
        } else {
            "n/a".to_string()
        }
    } else {
        format!("{:+.1}%", (b - a) / a * 100.0)
    }
}

/// Display label of a register index with the user annotation from
/// `--reg-names` when available: `0xA2F` or `0xA2F(tRCD)`. Shared by
/// the text view and the compare view of the strap "other fields".
pub fn reg_names_label(names: Option<&std::collections::HashMap<u16, String>>, idx: u16) -> String {
    if let Some(name) = names.and_then(|n| n.get(&idx)) {
        format!("0x{idx:X}({name})")
    } else {
        format!("0x{idx:X}")
    }
}

/// One group of "other fields": register label (or "RAW") + its
/// (field, value) pairs.
pub type RegGroup = (String, Vec<(String, String)>);

/// Decoded value of one timing field of one ROM at one clock, when the
/// strap list contains a strap at that clock.
pub fn field_at(rom: &ParsedRom, clk: i64, field: &str) -> Option<u32> {
    let strap = rom.vram.straps.find_by_clock_key(clk)?;
    strap.values.iter().enumerate().find_map(|(i, v)| {
        let idx = rom.vram.strap_reg_indices.get(i)?;
        let reg = crate::rom::timings::register(*idx)?;
        reg.fields
            .iter()
            .find(|f| f.name == field)
            .map(|f| (v >> f.offset) & ((1 << f.width) - 1))
    })
}

/// Renders a timing x clock matrix: rows are timing fields, columns
/// are clocks. `cell` provides each cell's content plus whether it
/// differs from the reference; rows with no differing cell are hidden
/// under `diff_only`, and with `color` set, differing cells are `warn`
/// and equal ones `good`.
pub fn core_matrix(
    pal: &Palette,
    rows: &[&str],
    clocks: &[i64],
    cell_w: usize,
    color: bool,
    diff_only: bool,
    cell: impl Fn(&str, i64) -> (String, bool),
) -> String {
    let mut out = String::new();
    out.push_str(&pal.label(&format!(
        "  {:<24} {}\n",
        "timing",
        clocks
            .iter()
            .map(|clk| format!("{clk:<cell_w$}"))
            .collect::<Vec<_>>()
            .join("")
    )));
    for field in rows {
        let mut cells = String::new();
        let mut row_differs = false;
        for clk in clocks {
            let (content, differs) = cell(field, *clk);
            row_differs |= differs;
            let padded = format!("{content:<cell_w$}");
            let colored = if color && differs {
                pal.warn(&padded)
            } else if color {
                pal.good(&padded)
            } else {
                padded
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
        out.push_str(&format!("  {label} {cells}\n"));
    }
    out
}

/// Collects non-empty part numbers from VRAM modules.
pub fn part_numbers(modules: &[crate::rom::types::VramModule]) -> Vec<String> {
    modules
        .iter()
        .filter(|m| !m.part_number.is_empty())
        .map(|m| m.part_number.clone())
        .collect()
}

/// Groups the non-core timing fields of one strap into per-register
/// entries (register name + offset), plus a single "RAW" entry holding
/// the raw hex of registers with no known timing layout. `unknown_label`
/// resolves the display label of one such register (e.g. a user
/// annotation from --reg-names).
pub fn strap_other_groups(
    values: &[u32],
    indices: &[u16],
    core_timings: &[&str],
    unknown_label: impl Fn(u16) -> String,
) -> Vec<RegGroup> {
    let mut groups: Vec<RegGroup> = Vec::new();
    let mut raw: Vec<(String, String)> = Vec::new();
    for (i, v) in values.iter().enumerate() {
        let Some(idx) = indices.get(i) else {
            continue;
        };
        match crate::rom::timings::register(*idx) {
            Some(reg) => {
                let fields: Vec<(String, String)> = reg
                    .fields
                    .iter()
                    .filter(|f| !core_timings.contains(&f.name))
                    .map(|f| {
                        (
                            f.name.to_string(),
                            ((v >> f.offset) & ((1 << f.width) - 1)).to_string(),
                        )
                    })
                    .collect();
                if !fields.is_empty() {
                    groups.push((format!("{} (0x{:X})", reg.name, idx), fields));
                }
            }
            None => raw.push((unknown_label(*idx), format!("0x{v:08X}"))),
        }
    }
    if !raw.is_empty() {
        groups.push(("RAW".to_string(), raw));
    }
    groups
}

/// Builds the colored "other fields" lines of one clock from the
/// per-ROM group lists. Registers are paired by identity (label),
/// fields positionally within a register. Returns `(identical, lines)`:
/// `identical` is true when every field is equal and present in all
/// ROMs. Fields equal across ROMs are shown in `good`, differing ones
/// in `warn`, `·` = absent.
pub fn other_fields_lines(pal: &Palette, per_rom: &[Vec<RegGroup>]) -> (bool, Vec<String>) {
    let mut labels: Vec<String> = Vec::new();
    for g in per_rom {
        for (label, _) in g {
            if !labels.contains(label) {
                labels.push(label.clone());
            }
        }
    }
    let mut lines: Vec<String> = Vec::new();
    let mut identical = true;
    for label in &labels {
        let per_rom_fields: Vec<Option<&Vec<(String, String)>>> = per_rom
            .iter()
            .map(|g| g.iter().find(|(l, _)| l == label).map(|(_, f)| f))
            .collect();
        let mut fields: Vec<String> = Vec::new();
        for f in per_rom_fields.iter().filter_map(|x| *x) {
            for (name, _) in f {
                if !fields.contains(name) {
                    fields.push(name.clone());
                }
            }
        }
        let mut parts: Vec<String> = Vec::new();
        for name in &fields {
            let vals: Vec<Option<&str>> = per_rom_fields
                .iter()
                .map(|p| {
                    p.and_then(|f| f.iter().find(|(n, _)| n == name))
                        .map(|(_, v)| v.as_str())
                })
                .collect();
            let present: Vec<&str> = vals.iter().filter_map(|x| *x).collect();
            let all_equal = !present.is_empty()
                && present.len() == vals.len()
                && present.windows(2).all(|w| w[0] == w[1]);
            if !all_equal {
                identical = false;
            }
            let joined = vals
                .iter()
                .map(|v| v.map(|s| s.to_string()).unwrap_or_else(|| "·".to_string()))
                .collect::<Vec<_>>()
                .join("/");
            let colored = if all_equal {
                pal.good(&format!("{name}={joined}"))
            } else {
                pal.warn(&format!("{name}={joined}"))
            };
            parts.push(colored);
        }
        if !parts.is_empty() {
            lines.push(format!("{label}: {}", parts.join(" ")));
        }
    }
    (identical, lines)
}

/// Pads the register labels so the values of all lines start at the
/// same column, and colors the label area (padding is applied to the
/// plain text before coloring, so ANSI codes never skew the width).
pub fn align_other_lines(pal: &Palette, lines: &[String]) -> Vec<String> {
    let max_label = lines
        .iter()
        .map(|l| l.find(':').map(|i| i + 1).unwrap_or(l.len()))
        .max()
        .unwrap_or(0);
    lines
        .iter()
        .map(|line| {
            let colon = line.find(':').map(|i| i + 1).unwrap_or(line.len());
            let (label, rest) = line.split_at(colon);
            let padded = format!("{label:<max_label$}");
            format!("    {}{}", pal.label(&padded), rest)
        })
        .collect()
}
