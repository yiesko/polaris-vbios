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
/// output. Only PowerPlay format 7 images can be named; the die is
/// detected from device ID + bootup message + MC microcode (Polaris 30
/// vs 20 vs 10 all share device 0x67DF, so the device ID alone would
/// mislabel RX 570/580/590), and images without a recognized Polaris
/// device ID get the generic label.
fn family_label(rom: &rom::types::ParsedRom) -> String {
    if rom.powerplay.header_fmt_rev == 7 {
        match rom::limits::detect_die(rom) {
            rom::limits::Die::Unknown => rom
                .pci_images
                .first()
                .and_then(|img| {
                    rom::validate::die_for_device_id(img.device_id).map(|(n, _)| n.to_string())
                })
                .unwrap_or_else(|| "Polaris/Tonga/Fiji".to_string()),
            die => die.label().to_string(),
        }
    } else {
        "unrecognized family".to_string()
    }
}

struct IdentifyData {
    vendor: String,
    family: String,
    vram_size_mb: u32,
    memory_types: Vec<String>,
    memory_vendors: Vec<String>,
    boost_sclk_mhz: f64,
    boost_mclk_mhz: f64,
    tdp_w: u16,
}

fn extract_identify_data(rom: &rom::types::ParsedRom) -> IdentifyData {
    let vram_size_mb: u32 = rom
        .vram
        .modules
        .iter()
        .map(|m| m.memory_size_mb as u32)
        .max()
        .unwrap_or(0);
    let mut memory_types: Vec<String> = rom
        .vram
        .modules
        .iter()
        .map(|m| m.memory_type_name.clone())
        .collect();
    memory_types.sort();
    memory_types.dedup();
    let mut memory_vendors: Vec<String> = rom
        .vram
        .modules
        .iter()
        .filter_map(|m| guess_memory_vendor(&m.part_number))
        .map(str::to_string)
        .collect();
    memory_vendors.sort();
    memory_vendors.dedup();
    IdentifyData {
        vendor: rom
            .header
            .subsystem_vendor_name
            .clone()
            .unwrap_or_else(|| format!("0x{:04X}", rom.header.subsystem_vendor_id)),
        family: family_label(rom),
        vram_size_mb,
        memory_types,
        memory_vendors,
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
    }
}

fn identify_json(rom: &rom::types::ParsedRom) -> IdentifyJson {
    let d = extract_identify_data(rom);
    IdentifyJson {
        file: rom.file_name.clone(),
        vendor: d.vendor,
        family: d.family,
        vram_size_mb: d.vram_size_mb,
        memory_types: d.memory_types,
        memory_vendors: d.memory_vendors,
        boost_sclk_mhz: d.boost_sclk_mhz,
        boost_mclk_mhz: d.boost_mclk_mhz,
        tdp_w: d.tdp_w,
        warnings: rom.warnings.clone(),
    }
}

fn identify_line(rom: &rom::types::ParsedRom, pal: &Palette) -> String {
    let d = extract_identify_data(rom);

    let vram_str = if d.vram_size_mb > 0 {
        format!(
            "{}MB {}{}",
            d.vram_size_mb,
            d.memory_types.join("/"),
            if d.memory_vendors.is_empty() {
                String::new()
            } else {
                format!(" ({})", d.memory_vendors.join("+"))
            }
        )
    } else {
        "VRAM: ?".to_string()
    };

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
        "{} [{}] {:<20} {vram_str:<28} boost {:.0}/{:.0}MHz  TDP {}W{bootup_col}  {status}",
        file_column(&rom.file_name, pal),
        d.vendor,
        d.family,
        d.boost_sclk_mhz,
        d.boost_mclk_mhz,
        d.tdp_w,
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
