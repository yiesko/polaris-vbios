//! `Command` handlers: one module per CLI subcommand plus shared
//! helpers used by several of them (exit codes, output writing, ROM
//! parsing, register-name loading).

mod check;
mod compare;
mod convert;
mod decode;
mod disasm;
mod dump;
mod extract;
mod identify;
mod patch;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::cli::Command;
use crate::rom;
use crate::rom::types::ParsedRom;

pub const EXIT_OK: u8 = 0;
pub const EXIT_ERROR: u8 = 1;
pub const EXIT_WARNINGS: u8 = 2;

/// Runs the given subcommand, mirroring the old `main.rs::run`.
pub fn run(cmd: Command) -> ExitCode {
    let sections = match cmd.sections() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(EXIT_ERROR);
        }
    };
    let json = cmd.json();
    let csv = match cmd.csv() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(EXIT_ERROR);
        }
    };
    let color = cmd.color();
    let output = cmd.output().cloned();
    let diff_only = cmd.diff_only();
    let reg_names = cmd.reg_names().cloned();

    match cmd {
        Command::Dump { roms, .. } => {
            dump::run(roms, sections, json, csv, color, output, reg_names)
        }
        Command::Compare { rom_a, rom_b, .. } => compare::run(
            rom_a, rom_b, sections, json, color, output, diff_only, reg_names,
        ),
        Command::CompareAll { roms, .. } => {
            compare::run_all(roms, sections, json, color, output, diff_only)
        }
        Command::Identify { roms, json, .. } => identify::run(roms, color, json),
        Command::Check { roms, quiet } => check::run(&roms, quiet),
        Command::Convert { clock, cycles, ns } => convert::run(clock, cycles, ns),
        Command::DecodeStrap {
            clock,
            values,
            indices,
        } => decode::run(clock, &values, indices.as_deref()),
        Command::Extract {
            rom,
            image,
            output,
            json,
        } => extract::run(&rom, &image, &output, json),
        Command::Tui { rom_a, rom_b } => match crate::tui::run(rom_a, rom_b) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("error: {e:#}");
                ExitCode::FAILURE
            }
        },
        Command::Disasm {
            rom,
            table,
            reg_names,
            no_color,
        } => disasm::run_disasm(&rom, table.as_deref(), reg_names.as_ref(), !no_color),
        Command::DiffDisasm {
            rom_a,
            rom_b,
            table,
            all,
            diff_only,
            reg_names,
            no_color,
        } => disasm::run_diff_disasm(
            &rom_a,
            &rom_b,
            table.as_deref(),
            all,
            diff_only,
            reg_names.as_ref(),
            !no_color,
        ),
        Command::Patch {
            rom,
            out,
            dry_run,
            fix_checksum,
            set_strap,
            set_strap_reg,
            retag_strap,
            timing,
            pp_sclk,
            pp_mclk,
            pp_vddc,
            pp_tdp,
            hex,
            clone_ids,
            import_vram,
            vram_size_mb,
            i_understand_strap_mismatch,
        } => patch::run(
            &rom,
            &out,
            dry_run,
            fix_checksum,
            set_strap,
            set_strap_reg,
            retag_strap,
            timing,
            pp_sclk,
            pp_mclk,
            pp_vddc,
            pp_tdp,
            hex,
            clone_ids,
            import_vram,
            vram_size_mb,
            i_understand_strap_mismatch,
        ),
        Command::ListSections | Command::Completions { .. } | Command::Man => unreachable!(),
    }
}

pub fn final_exit_code(had_error: bool, had_warnings: bool) -> ExitCode {
    if had_error {
        ExitCode::from(EXIT_ERROR)
    } else if had_warnings {
        ExitCode::from(EXIT_WARNINGS)
    } else {
        ExitCode::from(EXIT_OK)
    }
}

pub fn write_output(output: Option<PathBuf>, content: &str) -> bool {
    match output {
        Some(path) => match fs::write(&path, content) {
            Ok(()) => {
                eprintln!("saved to {}", path.display());
                true
            }
            Err(e) => {
                eprintln!("error writing to '{}': {e}", path.display());
                false
            }
        },
        None => {
            println!("{content}");
            true
        }
    }
}

pub fn load_reg_names(path: Option<&PathBuf>) -> Result<Option<HashMap<u16, String>>, ExitCode> {
    match path {
        None => Ok(None),
        Some(p) => match crate::reg_names::load(p) {
            Ok(map) => Ok(Some(map)),
            Err(e) => {
                eprintln!("error loading --reg-names: {e}");
                Err(ExitCode::from(EXIT_ERROR))
            }
        },
    }
}

/// Reads a ROM file, printing the usual "error reading" message on
/// failure. Shared by several subcommands so the read+report path is
/// identical everywhere.
pub fn read_rom(path: &Path) -> Result<Vec<u8>, ExitCode> {
    match fs::read(path) {
        Ok(d) => Ok(d),
        Err(e) => {
            eprintln!("error reading '{}': {e}", path.display());
            Err(ExitCode::from(EXIT_ERROR))
        }
    }
}

pub fn parse_roms(roms: &[PathBuf]) -> (Vec<ParsedRom>, bool) {
    let mut parsed = Vec::new();
    let mut had_error = false;
    for path in roms {
        match rom::parse_rom(path) {
            Ok(p) => parsed.push(p),
            Err(e) => {
                eprintln!("error reading '{}': {e:#}", path.display());
                had_error = true;
            }
        }
    }
    (parsed, had_error)
}

pub fn parse_rom_or_exit(path: &Path) -> Result<ParsedRom, ExitCode> {
    match rom::parse_rom(path) {
        Ok(p) => Ok(p),
        Err(e) => {
            eprintln!("error reading '{}': {e:#}", path.display());
            Err(ExitCode::from(EXIT_ERROR))
        }
    }
}

pub fn parse_num<T: std::str::FromStr>(s: &str, what: &str) -> Result<T, String> {
    let s = s.trim();
    let digits = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(s);
    digits
        .parse::<T>()
        .map_err(|_| format!("cannot parse '{s}' as {what}"))
}

pub fn parse_hex_bytes(token: &str, what: &str) -> Result<Vec<u8>, String> {
    token
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|t| !t.is_empty())
        .map(|t| {
            let digits = t
                .strip_prefix("0x")
                .or_else(|| t.strip_prefix("0X"))
                .unwrap_or(t);
            u8::from_str_radix(digits, 16)
                .map_err(|_| format!("cannot parse '{t}' as a hex byte in {what}"))
        })
        .collect()
}

pub fn parse_u32_hex(s: &str, what: &str) -> Result<u32, String> {
    let s = s.trim();
    let digits = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(s);
    u32::from_str_radix(digits, 16)
        .map_err(|_| format!("cannot parse '{s}' as a hex value for {what}"))
}
