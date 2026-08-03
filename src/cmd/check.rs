//! `check` subcommand: run every validation rule, scriptable exit codes.

use std::path::PathBuf;
use std::process::ExitCode;

use crate::{cmd, rom};

/// Exit codes are the scripting contract: 0 when every ROM parses
/// cleanly, 1 when findings are reported, 2 when a ROM fails to parse.
/// Deliberately different values from `EXIT_ERROR`/`EXIT_WARNINGS` -
/// the caller distinguishes "has findings" from "could not even read".
pub const EXIT_FINDINGS: u8 = 1;
pub const EXIT_PARSE_ERROR: u8 = 2;

pub fn run(roms: &[PathBuf], quiet: bool) -> ExitCode {
    let mut findings = false;
    let mut parse_error = false;
    for path in roms {
        match rom::parse_rom(path) {
            Ok(p) => {
                if !quiet && !p.warnings.is_empty() {
                    println!("{}", path.display());
                    for w in &p.warnings {
                        println!("  {w}");
                    }
                }
                findings |= !p.warnings.is_empty();
            }
            Err(e) => {
                if !quiet {
                    eprintln!("{}: error - {e:#}", path.display());
                }
                parse_error = true;
            }
        }
    }
    if parse_error {
        ExitCode::from(EXIT_PARSE_ERROR)
    } else if findings {
        ExitCode::from(EXIT_FINDINGS)
    } else {
        ExitCode::from(cmd::EXIT_OK)
    }
}
