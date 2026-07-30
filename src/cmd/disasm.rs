//! `disasm` and `diff-disasm` subcommands.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::render::color::Palette;
use crate::rom;
use crate::{cmd, diff_disasm};

pub fn run_disasm(
    rom_path: &Path,
    table_filter: Option<&str>,
    reg_names_path: Option<&PathBuf>,
    color: bool,
) -> ExitCode {
    let data = match cmd::read_rom(rom_path) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let r = rom::reader::Reader::new(&data);
    let header = match rom::header::parse_rom_header(&r) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("error: {e:#}");
            return ExitCode::from(cmd::EXIT_ERROR);
        }
    };
    let reg_names = match cmd::load_reg_names(reg_names_path) {
        Ok(n) => n,
        Err(code) => return code,
    };
    let tables = match rom::disasm::disasm_command_tables(
        &r,
        header.master_cmd_table_offset,
        table_filter,
        reg_names.as_ref(),
    ) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: {e:#}");
            return ExitCode::from(cmd::EXIT_ERROR);
        }
    };
    if tables.is_empty() {
        eprintln!("no command tables found in '{}'", rom_path.display());
        return ExitCode::from(cmd::EXIT_ERROR);
    }
    let pal = Palette::new(color);
    for t in &tables {
        println!(
            "\n{}",
            pal.title(&format!(
                "── {index:02} {name} (rev {fmt}.{cont}, ws {ws}, ps {ps}, {size} bytes) @ 0x{off:X} ",
                index = t.index,
                name = t.name,
                fmt = t.fmt_rev,
                cont = t.cont_rev,
                ws = t.ws_size,
                ps = t.ps_size,
                size = t.size,
                off = t.offset,
            ))
        );
        for l in &t.lines {
            println!("  {addr:04X}  {text}", addr = l.addr, text = l.text);
        }
    }
    ExitCode::from(cmd::EXIT_OK)
}

pub fn run_diff_disasm(
    a_path: &Path,
    b_path: &Path,
    table: Option<&str>,
    all: bool,
    diff_only: bool,
    reg_names_path: Option<&PathBuf>,
    color: bool,
) -> ExitCode {
    let _ = all; // default (no --table) is all tables, per spec
    let reg_names = match cmd::load_reg_names(reg_names_path) {
        Ok(n) => n,
        Err(code) => return code,
    };
    let content =
        match diff_disasm::run(a_path, b_path, table, diff_only, color, reg_names.as_ref()) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: {e:#}");
                return ExitCode::from(cmd::EXIT_ERROR);
            }
        };
    print!("{content}");
    ExitCode::from(cmd::EXIT_OK)
}
