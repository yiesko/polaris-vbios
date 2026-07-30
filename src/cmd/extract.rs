//! `extract` subcommand: dump PCIR chip images (EFI/legacy) to files.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::{cmd, rom};

pub fn run(rom_path: &Path, image_sel: &str, out_dir: &Path, json: bool) -> ExitCode {
    let data = match cmd::read_rom(rom_path) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let r = rom::reader::Reader::new(&data);
    let images = match rom::pci::walk_pci_images(&r) {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "error walking the PCIR chain in '{}': {e:#}",
                rom_path.display()
            );
            return ExitCode::from(cmd::EXIT_ERROR);
        }
    };
    if images.is_empty() {
        eprintln!("no PCIR images found in '{}'", rom_path.display());
        return ExitCode::from(cmd::EXIT_ERROR);
    }

    let selected: Vec<&rom::types::PciImage> = match image_sel.to_ascii_lowercase().as_str() {
        "all" => images.iter().collect(),
        "efi" => images.iter().filter(|i| i.code_type == 0x03).collect(),
        "legacy" => images.iter().filter(|i| i.code_type == 0x00).collect(),
        other => {
            eprintln!("error: unknown --image '{other}' (use 'efi', 'legacy' or 'all')");
            return ExitCode::from(cmd::EXIT_ERROR);
        }
    };
    if selected.is_empty() {
        eprintln!("no {image_sel} image found in '{}'", rom_path.display());
        return ExitCode::from(cmd::EXIT_ERROR);
    }

    let out_dir: &Path = out_dir;
    if json {
        match serde_json::to_string_pretty(&selected) {
            Ok(s) => {
                println!("{s}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("error generating JSON: {e}");
                ExitCode::from(cmd::EXIT_ERROR)
            }
        }
    } else {
        if let Err(e) = fs::create_dir_all(out_dir) {
            eprintln!(
                "error creating output directory '{}': {e}",
                out_dir.display()
            );
            return ExitCode::from(cmd::EXIT_ERROR);
        }
        let stem = rom_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "rom".to_string());
        let mut ok = true;
        for img in &selected {
            let type_name = match img.code_type {
                0x00 => "legacy",
                0x03 => "efi",
                _ => "image",
            };
            let name = format!("{stem}.{}-{type_name}.bin", img.index);
            let path: PathBuf = out_dir.join(name);
            let end = (img.file_offset + img.declared_size_bytes).min(data.len());
            match fs::write(&path, &data[img.file_offset..end]) {
                Ok(()) => println!(
                    "wrote {} ({} bytes, {} bytes available in file)",
                    path.display(),
                    end - img.file_offset,
                    img.declared_size_bytes
                ),
                Err(e) => {
                    eprintln!("error writing '{}': {e}", path.display());
                    ok = false;
                }
            }
        }
        if ok {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(cmd::EXIT_ERROR)
        }
    }
}
