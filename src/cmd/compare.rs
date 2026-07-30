//! `compare` and `compare-all` subcommands.

use std::path::PathBuf;
use std::process::ExitCode;

use crate::render::color::Palette;
use crate::render::sections::Section;
use crate::{cmd, compare, compare_all};

#[allow(clippy::too_many_arguments)]
pub fn run(
    rom_a_path: PathBuf,
    rom_b_path: PathBuf,
    sections: Vec<Section>,
    json: bool,
    color: bool,
    output: Option<PathBuf>,
    diff_only: bool,
    reg_names_path: Option<PathBuf>,
) -> ExitCode {
    let reg_names = match cmd::load_reg_names(reg_names_path.as_ref()) {
        Ok(r) => r,
        Err(code) => return code,
    };

    let a = match cmd::parse_rom_or_exit(&rom_a_path) {
        Ok(p) => p,
        Err(code) => return code,
    };
    let b = match cmd::parse_rom_or_exit(&rom_b_path) {
        Ok(p) => p,
        Err(code) => return code,
    };
    let had_warnings = !a.warnings.is_empty() || !b.warnings.is_empty();

    let content = if json {
        #[derive(serde::Serialize)]
        struct Pair<'a> {
            a: &'a crate::rom::types::ParsedRom,
            b: &'a crate::rom::types::ParsedRom,
        }
        match serde_json::to_string_pretty(&Pair { a: &a, b: &b }) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error generating JSON: {e}");
                return ExitCode::from(cmd::EXIT_ERROR);
            }
        }
    } else {
        let pal = Palette::new(color && output.is_none());
        let mut out = String::new();
        for (label, r) in [("A", &a), ("B", &b)] {
            if let Some(w) = crate::render::text::render_warnings(r, &pal) {
                out.push_str(&format!(
                    "{} ({}):\n{}\n\n",
                    pal.accent(&format!("ROM {label}")),
                    r.file_name,
                    w
                ));
            }
        }
        out.push_str(&compare::render_compare(
            &a,
            &b,
            &sections,
            &pal,
            diff_only,
            reg_names.as_ref(),
        ));
        out
    };

    let write_ok = cmd::write_output(output, &content);
    cmd::final_exit_code(!write_ok, had_warnings)
}

pub fn run_all(
    roms: Vec<PathBuf>,
    sections: Vec<Section>,
    json: bool,
    color: bool,
    output: Option<PathBuf>,
    diff_only: bool,
) -> ExitCode {
    let (parsed, had_error) = cmd::parse_roms(&roms);
    if parsed.len() < 2 {
        eprintln!("error: need at least 2 valid ROMs to compare");
        return ExitCode::from(cmd::EXIT_ERROR);
    }
    let had_warnings = parsed.iter().any(|p| !p.warnings.is_empty());

    let content = if json {
        match serde_json::to_string_pretty(&parsed) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error generating JSON: {e}");
                return ExitCode::from(cmd::EXIT_ERROR);
            }
        }
    } else {
        let pal = Palette::new(color && output.is_none());
        compare_all::render_compare_all(&parsed, &sections, &pal, diff_only)
    };

    let write_ok = cmd::write_output(output, &content);
    cmd::final_exit_code(had_error || !write_ok, had_warnings)
}
