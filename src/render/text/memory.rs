use super::{RegNames, heading, kv};
use crate::compare_util;
use crate::render::color::Palette;
use crate::render::sections::Section;
use crate::rom::types::ParsedRom;

pub(super) fn render_vram(rom: &ParsedRom, pal: &Palette) -> String {
    let v = &rom.vram;
    let mut s = heading(pal, Section::Vram.label());
    s.push('\n');
    s.push_str(&kv(pal, "Modules declared in ROM", v.num_modules));
    for m in &v.modules {
        s.push('\n');
        let pn = if m.part_number.is_empty() {
            pal.label("(empty / placeholder)")
        } else {
            pal.good(&m.part_number)
        };
        s.push_str(&format!(
            "  module {}: {}  ·  {} MB  ·  {}  ·  {} channel(s)  ·  vendor_id raw={}",
            m.index, pn, m.memory_size_mb, m.memory_type_name, m.channel_num, m.vendor_id_raw
        ));
    }
    if let Some(ver) = v.mcu_code_version {
        s.push('\n');
        s.push_str(&kv(
            pal,
            "MC ucode version",
            format!(
                "{ver} (rom start 0x{:X}, length {} bytes)",
                v.mcu_code_rom_start_addr.unwrap_or(0),
                v.mcu_code_length.unwrap_or(0)
            ),
        ));
    }
    s
}

pub(super) fn render_straps(rom: &ParsedRom, pal: &Palette, reg_names: RegNames) -> String {
    let v = &rom.vram;
    let mut s = heading(pal, Section::Straps.label());
    if v.straps.is_empty() {
        s.push_str("\n  (strap table not present in this ROM)");
        return s;
    }
    s.push('\n');
    s.push_str(&pal.label(&format!(
        "MC registers per strap (indices, hex): {}\n",
        v.strap_reg_indices
            .iter()
            .map(|r| format!("0x{r:X}"))
            .collect::<Vec<_>>()
            .join(", ")
    )));
    s.push_str(&pal.label(
        "Timing values are in memory-clock cycles (ns in parentheses for tRC/tRFC/tRP/tRRD/tFAW); \
         registers without a known timing layout are shown as raw hex.\n",
    ));
    if let Some(names) = reg_names {
        let matched = v.strap_reg_indices.iter().any(|r| names.contains_key(r));
        if !matched {
            s.push_str(
                &pal.warn("(--reg-names loaded, but no index from this ROM matches the file)\n"),
            );
        }
    }

    // Everything outside the core timing set, grouped by the register
    // it lives in (name + offset); raw hex for registers with no known
    // layout, user annotations from --reg-names applied inline.
    let groups_at = |strap: &crate::rom::types::MemoryStrap| {
        crate::compare_util::strap_other_groups(
            &strap.values,
            &v.strap_reg_indices,
            crate::rom::timings::CORE_TIMINGS,
            |idx| {
                if let Some(name) = reg_names.and_then(|n| n.get(&idx)) {
                    format!("0x{idx:X}({name})")
                } else {
                    format!("0x{idx:X}")
                }
            },
        )
    };

    let mut by_block: std::collections::BTreeMap<u8, Vec<&crate::rom::types::MemoryStrap>> =
        std::collections::BTreeMap::new();
    for strap in &v.straps {
        by_block.entry(strap.mem_block_id).or_default().push(strap);
    }

    for (blk, straps) in by_block {
        let module_tag = v
            .modules
            .get(blk as usize)
            .map(|m| {
                format!(
                    " - module {} (`{}`)",
                    blk,
                    if m.part_number.is_empty() {
                        "?"
                    } else {
                        &m.part_number
                    }
                )
            })
            .unwrap_or_default();
        s.push('\n');
        s.push_str(&pal.title(&format!("Block {blk}{module_tag}")));
        s.push('\n');

        // Core timings x clock matrix: rows are the timing fields users
        // understand at a glance, columns are the straps of this block
        // (one per clock). "219 (110 ns)" is the widest cell.
        let cell_w = 12;
        let clocks: Vec<i64> = straps
            .iter()
            .map(|st| st.clock_mhz.round() as i64)
            .collect();
        let cell = |field: &str, clk: i64| -> (String, bool) {
            let content = match compare_util::field_at(rom, clk, field) {
                Some(cycles) if crate::rom::timings::CLASSIC_NS.contains(&field) => format!(
                    "{} ({} ns)",
                    cycles,
                    crate::rom::timings::ns(cycles, clk as f64).round() as u64
                ),
                Some(cycles) => cycles.to_string(),
                None => "·".to_string(),
            };
            (content, false)
        };
        s.push_str(&pal.label("  Core timings per clock (cycles; · = no strap at this clock):\n"));
        s.push_str(&compare_util::core_matrix(
            pal,
            crate::rom::timings::CORE_TIMINGS,
            &clocks,
            cell_w,
            false,
            false,
            cell,
        ));
        s.push_str(&pal.label("  (ns shown for tRC/tRFC/tRP/tRRD/tFAW)\n"));

        s.push_str(&pal.label("\n  Other fields & raw registers per clock:\n"));
        for st in &straps {
            s.push_str(&format!(
                "  {}\n",
                pal.value(&format!("{:.0} MHz", st.clock_mhz))
            ));
            let lines: Vec<String> = groups_at(st)
                .iter()
                .map(|(label, fields)| {
                    let joined = fields
                        .iter()
                        .map(|(name, value)| format!("{name}={value}"))
                        .collect::<Vec<_>>()
                        .join(" ");
                    format!("{label}: {joined}")
                })
                .collect();
            for line in compare_util::align_other_lines(pal, &lines) {
                s.push_str(&line);
                s.push('\n');
            }
        }
    }

    let trips = crate::rom::validate::cross_checks(rom);
    if !trips.is_empty() {
        s.push('\n');
        s.push_str(&pal.title("Hard limit cross-check (informational)"));
        s.push('\n');
        for t in &trips {
            s.push_str(&format!("  {} {}\n", pal.warn("⚠"), pal.label(t)));
        }
        s.push_str(&pal.label("  (the driver/SMC clamps to the limit - the ROM still boots)\n"));
    }
    s
}

pub(super) fn render_caps(rom: &ParsedRom, pal: &Palette) -> String {
    let pp = &rom.powerplay;
    let mut s = heading(pal, Section::Caps.label());
    s.push('\n');
    s.push_str(&kv(
        pal,
        "platform_caps (raw)",
        format!("0x{:X}", pp.platform_caps),
    ));
    if pp.platform_caps_decoded.is_empty() {
        s.push_str("\n  (no recognized flags active)");
    } else {
        for flag in &pp.platform_caps_decoded {
            s.push_str(&format!("\n  • {flag}"));
        }
    }
    s.push('\n');
    s.push_str(&kv(
        pal,
        "Max engine overdrive",
        format!("{:.0} MHz", pp.max_overdrive_engine_mhz),
    ));
    s.push('\n');
    s.push_str(&kv(
        pal,
        "Max memory overdrive",
        format!("{:.0} MHz", pp.max_overdrive_memory_mhz),
    ));
    s.push('\n');
    s.push_str(&kv(
        pal,
        "Power control limit",
        format!("{}%", pp.power_control_limit_pct),
    ));
    if let Some(tc) = &pp.thermal_controller {
        s.push('\n');
        s.push_str(&kv(
            pal,
            "Thermal controller",
            format!(
                "type {} ({}) · i2c line {} address 0x{:X}",
                tc.kind, tc.kind_name, tc.i2c_line, tc.i2c_addr
            ),
        ));
    }
    s
}
