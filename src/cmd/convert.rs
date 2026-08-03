//! `convert` subcommand: cycles <-> nanoseconds at a given clock.

use std::process::ExitCode;

use crate::{cmd, rom};

pub fn run(clock_mhz: f64, cycles: Option<f64>, ns: Option<f64>) -> ExitCode {
    match (cycles, ns) {
        (Some(c), None) => {
            if clock_mhz <= 0.0 {
                eprintln!("error: --clock must be positive");
                return ExitCode::from(cmd::EXIT_ERROR);
            }
            if c < 0.0 {
                eprintln!("error: --cycles must be positive");
                return ExitCode::from(cmd::EXIT_ERROR);
            }
            println!(
                "{c} cycles = {} ns at {clock_mhz} MHz",
                rom::timings::ns(c as u32, clock_mhz)
            );
            ExitCode::from(cmd::EXIT_OK)
        }
        (None, Some(n)) => {
            if clock_mhz <= 0.0 {
                eprintln!("error: --clock must be positive");
                return ExitCode::from(cmd::EXIT_ERROR);
            }
            if n <= 0.0 {
                eprintln!("error: --ns must be positive");
                return ExitCode::from(cmd::EXIT_ERROR);
            }
            let c = (n * clock_mhz / 1000.0).round() as u64;
            println!("{n} ns = {c} cycles at {clock_mhz} MHz");
            ExitCode::from(cmd::EXIT_OK)
        }
        _ => {
            eprintln!("error: give exactly one of --cycles or --ns");
            ExitCode::from(cmd::EXIT_ERROR)
        }
    }
}
