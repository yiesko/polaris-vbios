//! `identify` subcommand: one one-line summary per ROM.

use std::path::PathBuf;
use std::process::ExitCode;

use crate::render::color::{Palette, fit, truncate};
use crate::{cmd, rom};

pub fn run(roms: Vec<PathBuf>, color: bool) -> ExitCode {
    let pal = Palette::new(color);
    let mut had_error = false;
    let mut had_warnings = false;
    for path in &roms {
        match rom::parse_rom(path) {
            Ok(p) => {
                had_warnings |= !p.warnings.is_empty();
                println!("{}", identify_line(&p, &pal));
            }
            Err(e) => {
                eprintln!("{}: error reading - {e:#}", path.display());
                had_error = true;
            }
        }
    }
    cmd::final_exit_code(had_error, had_warnings)
}

fn identify_line(rom: &rom::types::ParsedRom, pal: &Palette) -> String {
    let family = if rom.powerplay.header_fmt_rev == 7 {
        rom.pci_images
            .first()
            .and_then(|img| {
                rom::validate::die_for_device_id(img.device_id).map(|(n, _)| n.to_string())
            })
            .unwrap_or_else(|| "Polaris/Tonga/Fiji".to_string())
    } else {
        "unrecognized family".to_string()
    };
    let vendor = rom
        .header
        .subsystem_vendor_name
        .clone()
        .unwrap_or_else(|| format!("0x{:04X}", rom.header.subsystem_vendor_id));

    let total_mb: u32 = rom
        .vram
        .modules
        .iter()
        .map(|m| m.memory_size_mb as u32)
        .max()
        .unwrap_or(0);
    let mem_types: Vec<&str> = {
        let mut v: Vec<&str> = rom
            .vram
            .modules
            .iter()
            .map(|m| m.memory_type_name.as_str())
            .collect();
        v.sort();
        v.dedup();
        v
    };
    let mem_vendors: Vec<&str> = {
        let mut v: Vec<&str> = rom
            .vram
            .modules
            .iter()
            .filter_map(|m| guess_memory_vendor(&m.part_number))
            .collect();
        v.sort();
        v.dedup();
        v
    };
    let vram_str = if total_mb > 0 {
        format!(
            "{total_mb}MB {}{}",
            mem_types.join("/"),
            if mem_vendors.is_empty() {
                String::new()
            } else {
                format!(" ({})", mem_vendors.join("+"))
            }
        )
    } else {
        "VRAM: ?".to_string()
    };

    let boost_sclk = rom
        .powerplay
        .sclk_table
        .last()
        .map(|e| e.sclk_mhz)
        .unwrap_or(0.0);
    let boost_mclk = rom
        .powerplay
        .mclk_table
        .last()
        .map(|e| e.mclk_mhz)
        .unwrap_or(0.0);
    let tdp = rom
        .powerplay
        .powertune
        .as_ref()
        .map(|p| p.tdp_w)
        .unwrap_or(0);

    let status = if rom.warnings.is_empty() {
        pal.good("✓")
    } else {
        pal.bad(&format!("⚠{}", rom.warnings.len()))
    };

    let bootup = rom
        .header
        .bios_bootup_message
        .as_deref()
        .unwrap_or_default();
    let bootup_col = if bootup.is_empty() {
        String::new()
    } else {
        format!("  {}", pal.value(&truncate(bootup, 46)))
    };

    format!(
        "{} [{}] {family:<20} {vram_str:<28} boost {boost_sclk:.0}/{boost_mclk:.0}MHz  TDP {tdp}W{bootup_col}  {status}",
        file_column(&rom.file_name, pal),
        vendor,
    )
}

/// The filename column, width 32. Padding/truncation always happens on
/// PLAIN text first - applying color BEFORE alignment would make the
/// ANSI escape bytes count toward the field width and push every
/// following column out of line.
fn file_column(name: &str, pal: &Palette) -> String {
    pal.value(&fit(name, 32))
}

fn guess_memory_vendor(part_number: &str) -> Option<&'static str> {
    let p = part_number.trim();
    if p.starts_with("H5G") {
        Some("Hynix")
    } else if p.starts_with('K') && p.len() > 1 && p.as_bytes()[1] == b'4' {
        Some("Samsung")
    } else if p.starts_with("EDW") || p.starts_with("W4") {
        Some("Elpida/Micron")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Visible length, ignoring ANSI escape sequences.
    fn visible(s: &str) -> usize {
        let mut in_esc = false;
        let mut n = 0;
        for c in s.chars() {
            if in_esc {
                if c.is_ascii_alphabetic() {
                    in_esc = false;
                }
                continue;
            }
            if c == '\u{1b}' {
                in_esc = true;
            } else {
                n += 1;
            }
        }
        n
    }

    #[test]
    fn long_filename_truncates_at_exactly_32_columns() {
        let pal = Palette::new(false);
        let name = "XFX GTR-S RX 580 Black Limited Edition OC Ultra Premium.rom";
        let col = file_column(name, &pal);
        assert_eq!(visible(&col), 32);
    }

    #[test]
    fn short_filename_pads_out_to_32_columns() {
        let pal = Palette::new(false);
        let col = file_column("RX570_original.rom", &pal);
        assert_eq!(visible(&col), 32);
        assert!(col.starts_with("RX570_original.rom"));
    }

    #[test]
    fn colored_filename_keeps_visible_width_32() {
        let pal = Palette::new(true);
        let col = file_column("very_long_vendor_product_model_name.rom", &pal);
        assert_eq!(
            visible(&col),
            32,
            "ANSI bytes must not count toward the width"
        );
    }
}
