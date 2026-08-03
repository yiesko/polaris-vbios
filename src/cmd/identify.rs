//! `identify` subcommand: one one-line summary per ROM.

use std::path::PathBuf;
use std::process::ExitCode;

use crate::render::color::{Palette, fit, truncate};
use crate::{cmd, rom};

pub fn run(roms: Vec<PathBuf>, color: bool, json: bool) -> ExitCode {
    if json {
        return run_json(&roms);
    }
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

#[derive(serde::Serialize)]
struct IdentifyJson {
    file: String,
    vendor: String,
    family: String,
    vram_size_mb: u32,
    memory_types: Vec<String>,
    memory_vendors: Vec<String>,
    boost_sclk_mhz: f64,
    boost_mclk_mhz: f64,
    tdp_w: u16,
    warnings: Vec<String>,
}

fn run_json(roms: &[PathBuf]) -> ExitCode {
    let mut had_error = false;
    let mut had_warnings = false;
    let mut out = Vec::new();
    for path in roms {
        match rom::parse_rom(path) {
            Ok(p) => {
                had_warnings |= !p.warnings.is_empty();
                out.push(identify_json(&p));
            }
            Err(e) => {
                eprintln!("{}: error reading - {e:#}", path.display());
                had_error = true;
            }
        }
    }
    match serde_json::to_string_pretty(&out) {
        Ok(s) => {
            println!("{s}");
            cmd::final_exit_code(had_error, had_warnings)
        }
        Err(e) => {
            eprintln!("error generating JSON: {e}");
            ExitCode::from(cmd::EXIT_ERROR)
        }
    }
}

/// Display name of the ROM's family, shared by the JSON and text
/// output. Only PowerPlay format 7 images can be named; the device ID
/// picks the die, and images without a recognized Polaris device ID
/// get the generic label.
fn family_label(rom: &rom::types::ParsedRom) -> String {
    if rom.powerplay.header_fmt_rev == 7 {
        rom.pci_images
            .first()
            .and_then(|img| {
                rom::validate::die_for_device_id(img.device_id).map(|(n, _)| n.to_string())
            })
            .unwrap_or_else(|| "Polaris/Tonga/Fiji".to_string())
    } else {
        "unrecognized family".to_string()
    }
}

fn identify_json(rom: &rom::types::ParsedRom) -> IdentifyJson {
    let total_mb: u32 = rom
        .vram
        .modules
        .iter()
        .map(|m| m.memory_size_mb as u32)
        .max()
        .unwrap_or(0);
    let mut mem_types: Vec<String> = rom
        .vram
        .modules
        .iter()
        .map(|m| m.memory_type_name.clone())
        .collect();
    mem_types.sort();
    mem_types.dedup();
    let mut mem_vendors: Vec<String> = rom
        .vram
        .modules
        .iter()
        .filter_map(|m| guess_memory_vendor(&m.part_number))
        .map(str::to_string)
        .collect();
    mem_vendors.sort();
    mem_vendors.dedup();
    IdentifyJson {
        file: rom.file_name.clone(),
        vendor: rom
            .header
            .subsystem_vendor_name
            .clone()
            .unwrap_or_else(|| format!("0x{:04X}", rom.header.subsystem_vendor_id)),
        family: family_label(rom),
        vram_size_mb: total_mb,
        memory_types: mem_types,
        memory_vendors: mem_vendors,
        boost_sclk_mhz: rom
            .powerplay
            .sclk_table
            .last()
            .map(|e| e.sclk_mhz)
            .unwrap_or(0.0),
        boost_mclk_mhz: rom
            .powerplay
            .mclk_table
            .last()
            .map(|e| e.mclk_mhz)
            .unwrap_or(0.0),
        tdp_w: rom
            .powerplay
            .powertune
            .as_ref()
            .map(|p| p.tdp_w)
            .unwrap_or(0),
        warnings: rom.warnings.clone(),
    }
}

fn identify_line(rom: &rom::types::ParsedRom, pal: &Palette) -> String {
    let family = family_label(rom);
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
