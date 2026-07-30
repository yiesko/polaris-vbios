//! `dump` subcommand: render the selected sections (text, CSV or JSON).

use std::path::PathBuf;
use std::process::ExitCode;

use crate::render::color::Palette;
use crate::render::sections::Section;
use crate::{cmd, csv_export, rom};

#[allow(clippy::too_many_arguments)]
pub fn run(
    roms: Vec<PathBuf>,
    sections: Vec<Section>,
    json: bool,
    csv: bool,
    color: bool,
    output: Option<PathBuf>,
    reg_names_path: Option<PathBuf>,
) -> ExitCode {
    let reg_names = match cmd::load_reg_names(reg_names_path.as_ref()) {
        Ok(r) => r,
        Err(code) => return code,
    };

    let (parsed, had_error) = cmd::parse_roms(&roms);
    if parsed.is_empty() {
        return ExitCode::from(cmd::EXIT_ERROR);
    }
    let had_warnings = parsed.iter().any(|p| !p.warnings.is_empty());

    let content = if csv {
        if sections.len() != 1 {
            eprintln!(
                "error: --format csv requires exactly one tabular section in --sections (got {}). Tabular sections: {}",
                sections.len(),
                csv_export::exportable_keys().join(", ")
            );
            return ExitCode::from(cmd::EXIT_ERROR);
        }
        let refs: Vec<&rom::types::ParsedRom> = parsed.iter().collect();
        match csv_export::export_csv(&refs, sections[0]) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::from(cmd::EXIT_ERROR);
            }
        }
    } else if json {
        let value = if parsed.len() == 1 {
            serde_json::to_string_pretty(&parsed[0])
        } else {
            serde_json::to_string_pretty(&parsed)
        };
        match value {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error generating JSON: {e}");
                return ExitCode::from(cmd::EXIT_ERROR);
            }
        }
    } else {
        let pal = Palette::new(color && output.is_none());
        parsed
            .iter()
            .map(|p| crate::render::text::render_sections(p, &sections, &pal, reg_names.as_ref()))
            .collect::<Vec<_>>()
            .join("\n\n")
    };

    let write_ok = cmd::write_output(output, &content);
    cmd::final_exit_code(had_error || !write_ok, had_warnings)
}
