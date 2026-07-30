pub mod cli;
pub mod clipboard;
pub mod cmd;
pub mod compare;
pub mod compare_all;
pub mod compare_util;
pub mod csv_export;
pub mod diff_disasm;
pub mod reg_names;
pub mod render;
pub mod rom;
pub mod tui;

use std::process::ExitCode;

use cli::Command;

/// Library entry point: parses the command line and dispatches to the
/// matching subcommand handler. The thin binary (`src/main.rs`) just
/// calls this.
pub fn run() -> ExitCode {
    match cli::parse() {
        Ok(Command::ListSections) => {
            cli::print_list_sections();
            ExitCode::SUCCESS
        }
        Ok(cmd) => cmd::run(cmd),
        Err(msg) => {
            eprintln!("{msg}");
            ExitCode::FAILURE
        }
    }
}
