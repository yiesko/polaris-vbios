use std::path::PathBuf;

use clap::{CommandFactory, Parser, Subcommand};

use crate::render::sections::{Section, parse_section_list};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(name = "polaris-vbios", version = VERSION, about = "AMD Polaris VBIOS reader/dumper/comparator")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// List available sections and exit
    #[arg(long, default_value_t = false)]
    pub list_sections: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Read one or more ROMs and display info in the terminal (or file/JSON/CSV)
    Dump {
        #[arg(required = true)]
        roms: Vec<PathBuf>,
        #[arg(long, default_value = "all")]
        sections: String,
        #[arg(long)]
        json: bool,
        /// Output format (csv or text; only for 'dump')
        #[arg(long)]
        format: Option<String>,
        /// Register annotation file for the 'straps' section
        #[arg(long)]
        reg_names: Option<PathBuf>,
        #[arg(long)]
        no_color: bool,
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,
    },
    /// Compare two ROMs side by side
    Compare {
        rom_a: PathBuf,
        rom_b: PathBuf,
        #[arg(long, default_value = "all")]
        sections: String,
        #[arg(long)]
        json: bool,
        /// Register annotation file for the 'straps' section
        #[arg(long)]
        reg_names: Option<PathBuf>,
        #[arg(long)]
        no_color: bool,
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,
        #[arg(long)]
        diff_only: bool,
    },
    /// Matrix comparison for 2+ ROMs at once
    CompareAll {
        #[arg(required = true)]
        roms: Vec<PathBuf>,
        #[arg(long, default_value = "all")]
        sections: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        no_color: bool,
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,
        #[arg(long)]
        diff_only: bool,
    },
    /// Summary mode: one line per ROM (ASIC, VRAM, boost clock, TDP...)
    Identify {
        #[arg(required = true)]
        roms: Vec<PathBuf>,
        #[arg(long)]
        no_color: bool,
    },
    /// Open the interactive TUI
    Tui {
        rom_a: PathBuf,
        rom_b: Option<PathBuf>,
    },
    /// Extract the firmware images (legacy x86 / UEFI GOP) from the
    /// PCIR chain to files, or list their metadata as JSON
    Extract {
        #[arg(required = true)]
        rom: PathBuf,
        /// Which image to extract: 'efi', 'legacy' or 'all'
        #[arg(long, default_value = "all")]
        image: String,
        /// Output directory (created if missing)
        #[arg(short = 'o', long, default_value = ".")]
        output: PathBuf,
        /// Print image metadata as JSON instead of writing files
        #[arg(long)]
        json: bool,
    },
    /// Disassemble the ATOM bytecode of the command tables
    Disasm {
        #[arg(required = true)]
        rom: PathBuf,
        /// Only disassemble this table (by name or index)
        #[arg(long)]
        table: Option<String>,
        /// Register annotation file (like the 'straps' section uses)
        #[arg(long)]
        reg_names: Option<PathBuf>,
        #[arg(long)]
        no_color: bool,
    },
    /// Diff the disassembly of two ROMs' command tables
    DiffDisasm {
        rom_a: PathBuf,
        rom_b: PathBuf,
        /// Only diff this table (by name or index)
        #[arg(long)]
        table: Option<String>,
        /// Diff all tables (default)
        #[arg(long)]
        all: bool,
        /// Show only lines that differ
        #[arg(long)]
        diff_only: bool,
        /// Register annotation file (like the 'straps' section uses)
        #[arg(long)]
        reg_names: Option<PathBuf>,
        #[arg(long)]
        no_color: bool,
    },
    /// Patch a ROM safely: apply edits, recompute the legacy checksum,
    /// verify the result, never write in place
    Patch {
        #[arg(required = true)]
        rom: PathBuf,
        /// Output ROM file (required - the source is never modified)
        #[arg(long, required = true)]
        out: PathBuf,
        /// Show the edit plan and verification, write nothing
        #[arg(long)]
        dry_run: bool,
        /// Recompute the legacy checksum; also allows patching an image
        /// whose input checksum is already invalid (repair)
        #[arg(long)]
        fix_checksum: bool,
        /// Set strap register: <clock_mhz> <reg_index> <value>
        #[arg(long, num_args = 3, value_names = ["CLOCK_MHZ", "REG_INDEX", "VALUE"])]
        set_strap: Vec<String>,
        /// Set MC register in every strap block: <reg_offset> <value>
        #[arg(long, num_args = 2, value_names = ["REG_OFFSET", "VALUE"])]
        set_strap_reg: Vec<String>,
        /// Change the clock a strap is tagged with: <clock_mhz> <new_mhz>.
        /// Refused above the highest strap clock the ROM ships (the MC
        /// only trains those); warned when no MCLK DPM level matches
        #[arg(long, num_args = 2, value_names = ["CLOCK_MHZ", "NEW_MHZ"])]
        retag_strap: Vec<String>,
        /// Set SCLK DPM level: <level> <mhz>
        #[arg(long, num_args = 2, value_names = ["LEVEL", "MHZ"])]
        pp_sclk: Vec<String>,
        /// Set MCLK DPM level: <level> <mhz>
        #[arg(long, num_args = 2, value_names = ["LEVEL", "MHZ"])]
        pp_mclk: Vec<String>,
        /// Set VDDC LUT entry: <index> <mv>
        #[arg(long, num_args = 2, value_names = ["INDEX", "MV"])]
        pp_vddc: Vec<String>,
        /// Set PowerTune TDP in watts
        #[arg(long, value_name = "WATTS")]
        pp_tdp: Vec<String>,
        /// Write raw bytes: <offset> <bytes> (bytes as a single hex
        /// string, repeatable). Separate the byte pairs with spaces
        /// and/or commas - both are accepted, e.g. "0A FF" or "0A,FF".
        /// Refused for boot/BIOS-data/table-header layout areas;
        /// unchecksummed or structure-overlapping writes are warned
        #[arg(long, num_args = 2, value_names = ["OFFSET", "BYTES"])]
        hex: Vec<String>,
    },
    /// List available sections
    ListSections,
    /// Print a shell completion script for your shell and exit
    Completions {
        /// Which shell to generate completions for:
        /// bash, elvish, fish, powershell or zsh
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    /// Print a roff man page and exit
    Man,
}

pub fn parse() -> Result<Command, String> {
    match Cli::try_parse() {
        Ok(cli) => {
            if cli.list_sections {
                return Ok(Command::ListSections);
            }
            match cli.command {
                Some(cmd) => Ok(cmd),
                None => {
                    let mut cmd = Cli::command();
                    cmd.print_help().ok();
                    println!();
                    std::process::exit(0);
                }
            }
        }
        Err(e) => match e.kind() {
            clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion => {
                println!("{e}");
                std::process::exit(0);
            }
            _ => Err(e.to_string()),
        },
    }
}

/// Emits a shell completion script for `shell` to stdout, then exits.
pub fn print_completions(shell: clap_complete::Shell) {
    let name = env!("CARGO_PKG_NAME");
    let mut cmd = Cli::command();
    clap_complete::generate(shell, &mut cmd, name, &mut std::io::stdout());
    std::process::exit(0);
}

/// Emits a roff man page for the CLI to stdout, then exits.
pub fn print_man() {
    let cmd = Cli::command();
    let man = clap_mangen::Man::new(cmd);
    let mut out = std::io::stdout();
    man.render(&mut out).ok();
    std::process::exit(0);
}

pub fn print_list_sections() {
    println!("Available sections (use with --sections, comma-separated):\n");
    for s in Section::ALL {
        println!("  {:<10} {}", s.key(), s.label());
    }
    println!("  {:<10} All sections above (default)", "all");
}

impl Command {
    pub fn sections(&self) -> Result<Vec<Section>, String> {
        match self {
            Command::Dump { sections, .. }
            | Command::Compare { sections, .. }
            | Command::CompareAll { sections, .. } => parse_section_list(sections),
            _ => Ok(Section::ALL.to_vec()),
        }
    }

    pub fn color(&self) -> bool {
        match self {
            Command::Dump { no_color, .. }
            | Command::Compare { no_color, .. }
            | Command::CompareAll { no_color, .. }
            | Command::Identify { no_color, .. }
            | Command::Disasm { no_color, .. }
            | Command::DiffDisasm { no_color, .. } => !no_color,
            Command::Tui { .. } => true,
            Command::Extract { .. } => true,
            Command::Patch { .. } => false,
            Command::ListSections => true,
            Command::Completions { .. } => true,
            Command::Man => true,
        }
    }

    pub fn output(&self) -> Option<&PathBuf> {
        match self {
            Command::Dump { output, .. }
            | Command::Compare { output, .. }
            | Command::CompareAll { output, .. } => output.as_ref(),
            _ => None,
        }
    }

    pub fn json(&self) -> bool {
        match self {
            Command::Dump { json, .. }
            | Command::Compare { json, .. }
            | Command::CompareAll { json, .. } => *json,
            _ => false,
        }
    }

    pub fn csv(&self) -> Result<bool, String> {
        match self {
            Command::Dump { format, json, .. } => {
                let csv = match format.as_deref() {
                    Some("csv") => true,
                    Some("text") => false,
                    Some(other) => {
                        return Err(format!("unknown format: '{other}' (use 'csv' or 'text')"));
                    }
                    None => false,
                };
                if csv && *json {
                    return Err("use --json OR --format csv, not both".to_string());
                }
                Ok(csv)
            }
            _ => Ok(false),
        }
    }

    pub fn diff_only(&self) -> bool {
        match self {
            Command::Compare { diff_only, .. } | Command::CompareAll { diff_only, .. } => {
                *diff_only
            }
            _ => false,
        }
    }

    pub fn reg_names(&self) -> Option<&PathBuf> {
        match self {
            Command::Dump { reg_names, .. } | Command::Compare { reg_names, .. } => {
                reg_names.as_ref()
            }
            _ => None,
        }
    }
}
