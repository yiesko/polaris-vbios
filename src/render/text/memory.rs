use super::{RegNames, heading, kv};
use crate::render::color::Palette;
use crate::render::color::pad;
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
        "Timing values below are in memory-clock cycles (ns in parentheses); registers without a known timing layout are shown as raw hex.\n",
    ));
    if let Some(names) = reg_names {
        let legend: Vec<String> = v
            .strap_reg_indices
            .iter()
            .enumerate()
            .filter_map(|(i, r)| names.get(r).map(|name| format!("reg{i}=0x{r:X}({name})")))
            .collect();
        if legend.is_empty() {
            s.push_str(
                &pal.warn("(--reg-names loaded, but no index from this ROM matches the file)\n"),
            );
        } else {
            s.push_str(&pal.good(&format!(
                "User annotations (--reg-names, not confirmed by AMD): {}\n",
                legend.join(", ")
            )));
        }
    }

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
        for strap in straps {
            s.push_str(&format!(
                "  {} {} {}\n",
                pal.value(&pad(&format!("{:.0} MHz", strap.clock_mhz), 10)),
                pad(&format!("({:.2} Gbps effective)", strap.effective_gbps), 14),
                crate::rom::timings::fmt_strap(
                    &strap.values,
                    &v.strap_reg_indices,
                    strap.clock_mhz
                )
            ));
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
