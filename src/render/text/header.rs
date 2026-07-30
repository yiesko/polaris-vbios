use super::{fmt_kib, heading, kv};
use crate::render::color::Palette;
use crate::render::sections::Section;
use crate::rom::types::ParsedRom;

pub(super) fn render_header(rom: &ParsedRom, pal: &Palette) -> String {
    let h = &rom.header;
    let mut s = heading(pal, Section::Header.label());
    s.push('\n');
    s.push_str(&kv(
        pal,
        "File size",
        format!(
            "{} bytes ({:.0} KiB)",
            h.file_size,
            h.file_size as f64 / 1024.0
        ),
    ));
    s.push('\n');
    s.push_str(&kv(
        pal,
        "ATOM header at",
        format!("0x{:X}", h.atom_header_offset),
    ));
    s.push('\n');
    s.push_str(&kv(
        pal,
        "ATOM format/revision",
        format!("{}.{}", h.atom_fmt_rev, h.atom_cont_rev),
    ));
    s.push('\n');
    s.push_str(&kv(
        pal,
        "Master Data Table",
        format!("0x{:X}", h.master_data_table_offset),
    ));
    s.push('\n');
    s.push_str(&kv(
        pal,
        "Master Command Table",
        format!("0x{:X}", h.master_cmd_table_offset),
    ));
    s.push('\n');
    s.push_str(&kv(
        pal,
        "PowerPlay table rev.",
        format!(
            "{} (format {}.{}) - {}",
            rom.powerplay.table_revision,
            rom.powerplay.header_fmt_rev,
            rom.powerplay.header_cont_rev,
            if rom.powerplay.header_fmt_rev == 7 {
                "Tonga/Fiji/Polaris family"
            } else {
                "unusual format for Polaris - check the warnings"
            }
        ),
    ));
    s.push('\n');
    let subsys = match &h.subsystem_vendor_name {
        Some(name) => format!(
            "{:04X}:{:04X} ({name})",
            h.subsystem_vendor_id, h.subsystem_device_id
        ),
        None => format!(
            "{:04X}:{:04X} (uncatalogued vendor)",
            h.subsystem_vendor_id, h.subsystem_device_id
        ),
    };
    s.push_str(&kv(pal, "Subsystem vendor:device", subsys));
    s.push('\n');
    let checksum_str = if h.checksum_valid {
        pal.good(&format!(
            "OK (sum 0x00 over {} bytes)",
            h.checksum_checked_bytes
        ))
    } else {
        pal.bad(&format!(
            "INVALID (sum 0x{:02X}, expected 0x00, over {} declared bytes)",
            h.checksum_computed_sum, h.checksum_checked_bytes
        ))
    };
    s.push_str(&format!(
        "  {}  {}",
        pal.label(&format!("{:<28}", "Checksum")),
        checksum_str
    ));
    if let Some(msg) = &h.bios_bootup_message {
        s.push('\n');
        s.push_str(&kv(pal, "BIOS bootup message", msg));
    }
    if let Some(cfg) = &h.config_filename {
        s.push('\n');
        s.push_str(&kv(pal, "Config filename (BIOS build)", cfg));
    }
    if !h.build_date_candidates.is_empty() {
        s.push('\n');
        s.push_str(&kv(
            pal,
            "Build date (heuristic)",
            h.build_date_candidates.join(" · "),
        ));
    }
    if !h.command_tables_present.is_empty() {
        s.push('\n');
        s.push_str(&format!(
            "  {} ({} of {} present)\n",
            pal.label("Command tables"),
            h.command_tables_present.len(),
            crate::rom::header::COMMAND_TABLE_NAMES.len()
        ));
        for name in &h.command_tables_present {
            s.push_str(&format!("    • {name}\n"));
        }
    }
    s
}

pub(super) fn render_firmware(rom: &ParsedRom, pal: &Palette) -> String {
    let f = &rom.firmware;
    let mut s = heading(pal, Section::Firmware.label());
    s.push('\n');
    s.push_str(&kv(
        pal,
        "Boot engine clock",
        format!("{:.0} MHz", f.default_engine_clock_mhz),
    ));
    s.push('\n');
    s.push_str(&kv(
        pal,
        "Boot memory clock",
        format!("{:.0} MHz", f.default_memory_clock_mhz),
    ));
    s.push('\n');
    s.push_str(&kv(
        pal,
        "Reference clock (core)",
        format!("{:.0} MHz", f.core_ref_clock_mhz),
    ));
    s.push('\n');
    s.push_str(&kv(
        pal,
        "Reference clock (mem)",
        format!("{:.0} MHz", f.mem_ref_clock_mhz),
    ));
    s.push('\n');
    s.push_str(&kv(pal, "Boot VDDC", format!("{} mV", f.bootup_vddc_mv)));
    s.push('\n');
    s.push_str(&kv(pal, "Boot VDDCI", format!("{} mV", f.bootup_vddci_mv)));
    if f.bootup_mvddc_mv > 0 {
        s.push('\n');
        s.push_str(&kv(pal, "Boot MVDDC", format!("{} mV", f.bootup_mvddc_mv)));
    }
    if f.bootup_vddgfx_mv > 0 {
        s.push('\n');
        s.push_str(&kv(
            pal,
            "Boot VDDGFX",
            format!("{} mV", f.bootup_vddgfx_mv),
        ));
    }
    if f.spll_output_mhz > 0.0 {
        s.push('\n');
        s.push_str(&kv(
            pal,
            "SPLL output freq",
            format!("{:.0} MHz", f.spll_output_mhz),
        ));
    }
    if f.gpull_output_mhz > 0.0 {
        s.push('\n');
        s.push_str(&kv(
            pal,
            "GPUPLL output freq",
            format!("{:.0} MHz", f.gpull_output_mhz),
        ));
    }
    if f.max_pixel_clock_pll_mhz > 0.0 {
        s.push('\n');
        s.push_str(&kv(
            pal,
            "Max pixel clock (PLL output)",
            format!("{:.0} MHz", f.max_pixel_clock_pll_mhz),
        ));
    }
    if f.default_disp_engine_clk_mhz > 0.0 {
        s.push('\n');
        s.push_str(&kv(
            pal,
            "Default display engine clock",
            format!("{:.0} MHz", f.default_disp_engine_clk_mhz),
        ));
    }
    if f.min_pixel_clock_pll_input_mhz > 0.0 || f.max_pixel_clock_pll_input_mhz > 0.0 {
        s.push('\n');
        s.push_str(&kv(
            pal,
            "Pixel clock PLL input range",
            format!(
                "{:.0}–{:.0} MHz",
                f.min_pixel_clock_pll_input_mhz, f.max_pixel_clock_pll_input_mhz
            ),
        ));
    }
    if f.min_pixel_clock_pll_output_mhz > 0.0 {
        s.push('\n');
        s.push_str(&kv(
            pal,
            "Min pixel clock (PLL output)",
            format!("{:.0} MHz", f.min_pixel_clock_pll_output_mhz),
        ));
    }
    if f.uniphy_dp_mode_ext_clk_mhz > 0.0 {
        s.push('\n');
        s.push_str(&kv(
            pal,
            "Uniphy DP mode ext clock",
            format!("{:.0} MHz", f.uniphy_dp_mode_ext_clk_mhz),
        ));
    }
    s.push('\n');
    s.push_str(&kv(
        pal,
        "Cooling solution",
        f.cooling_solution_name.clone(),
    ));
    if f.branding_id > 0 || f.embedded_cap > 0 {
        s.push('\n');
        s.push_str(&kv(
            pal,
            "Product branding",
            format!(
                "branding id 0x{:X} · embedded cap {}",
                f.branding_id, f.embedded_cap
            ),
        ));
    }
    for (i, v) in f.vram_reserves.iter().enumerate() {
        s.push('\n');
        let desc = if i == 0 && v.firmware_use_kb > 0 {
            " (BIOS firmware)"
        } else if i == 0 {
            " (driver framebuffer)"
        } else {
            ""
        };
        let start_mb = v.start_addr as f64 / (1024.0 * 1024.0);
        let mut parts = vec![format!("start 0x{:X} ({:.1} MiB)", v.start_addr, start_mb)];
        if v.firmware_use_kb > 0 {
            parts.push(format!("fw {}", fmt_kib(v.firmware_use_kb)));
        }
        if v.driver_use_kb > 0 {
            parts.push(format!("driver {}", fmt_kib(v.driver_use_kb)));
        }
        s.push_str(&kv(
            pal,
            &format!("VRAM reserve {i}"),
            format!("{}{}", parts.join(" · "), desc),
        ));
    }
    s
}
