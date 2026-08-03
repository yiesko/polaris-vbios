//! `decode-strap` subcommand: decode a pasted memory strap register set
//! without a ROM. Uses the default 14-slot Polaris register index table
//! unless `--indices` overrides it.

use std::process::ExitCode;

use crate::cmd;
use crate::rom::timings;

/// Parses a comma/space-separated list of hex register indices.
fn parse_indices(s: &str) -> Result<Vec<u16>, String> {
    s.split(|c: char| c == ',' || c.is_whitespace())
        .filter(|t| !t.is_empty())
        .map(|t| {
            let digits = t
                .strip_prefix("0x")
                .or_else(|| t.strip_prefix("0X"))
                .unwrap_or(t);
            u16::from_str_radix(digits, 16)
                .map_err(|_| format!("cannot parse '{t}' as a hex register index"))
        })
        .collect()
}

pub fn run(clock_mhz: f64, values: &[String], indices_arg: Option<&str>) -> ExitCode {
    if clock_mhz <= 0.0 {
        eprintln!("error: clock must be positive");
        return ExitCode::from(cmd::EXIT_ERROR);
    }
    let mut vals = Vec::new();
    for v in values {
        match cmd::parse_u32_hex(v, "decode-strap") {
            Ok(x) => vals.push(x),
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::from(cmd::EXIT_ERROR);
            }
        }
    }
    let indices = match indices_arg {
        Some(s) => match parse_indices(s) {
            Ok(i) => i,
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::from(cmd::EXIT_ERROR);
            }
        },
        None => timings::DEFAULT_INDICES.to_vec(),
    };
    if vals.len() > indices.len() {
        eprintln!(
            "error: {} register values but only {} register slots - give --indices with \
             more slots or fewer values",
            vals.len(),
            indices.len()
        );
        return ExitCode::from(cmd::EXIT_ERROR);
    }

    println!(
        "memory strap @ {clock_mhz} MHz ({} register slot(s), ns = cycles * 1000 / {clock_mhz})",
        indices.len()
    );

    // Core timings first - the numbers users plan edits with.
    for name in timings::CORE_TIMINGS {
        if let Some((reg, field)) = timings::field_named(name)
            && let Some(cycles) = indices
                .iter()
                .position(|i| *i == reg.index)
                .and_then(|slot| vals.get(slot))
                .copied()
        {
            let cycles = (cycles >> field.offset) & ((1 << field.width) - 1);
            if field.in_ns_set {
                println!(
                    "  {name:<7} = {cycles:>3} cycles  ({} ns)",
                    timings::ns(cycles, clock_mhz).round() as u64
                );
            } else {
                println!("  {name:<7} = {cycles:>3} cycles");
            }
        }
    }

    // The remaining fields grouped by register, plus raw slots.
    println!("  registers:");
    for (i, value) in vals.iter().enumerate() {
        let idx = indices.get(i).copied().unwrap_or(0);
        let line = match timings::register(idx) {
            Some(reg) => {
                let fields = reg
                    .fields
                    .iter()
                    .map(|f| {
                        let cycles = (value >> f.offset) & ((1 << f.width) - 1);
                        if f.in_ns_set {
                            format!(
                                "{}={} ({} ns)",
                                f.name,
                                cycles,
                                timings::ns(cycles, clock_mhz).round() as u64
                            )
                        } else {
                            format!("{}={}", f.name, cycles)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("    0x{idx:X}  {:<15} 0x{value:08X}  {fields}", reg.name)
            }
            None => format!("    0x{idx:X}  {:<15} 0x{value:08X}", ""),
        };
        println!("{line}");
    }
    ExitCode::from(cmd::EXIT_OK)
}
