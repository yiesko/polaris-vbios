//! Validate-then-apply patching: every requested edit is checked
//! against the ROM's own constraints (hard limits, die maximum,
//! structural layout) before anything is written. Applying never
//! mutates the input - ops produce [`PatchReport`], and
//! [`apply_diffs`] writes those diffs onto a buffer copy.

mod apply;
pub(super) mod checksum;
pub(super) mod limits;
mod map;
mod types;

pub use checksum::fix_checksum;
pub use types::{Diff, PatchOp, PatchReport};

use anyhow::Result;

use super::reader::Reader;

/// Returns human-readable notes for edit pairs that overlap in the
/// ROM (the later one silently wins during `apply_diffs`).
pub fn overlapping_diffs(diffs: &[Diff]) -> Vec<String> {
    let mut notes = Vec::new();
    for (i, a) in diffs.iter().enumerate() {
        for b in diffs.iter().skip(i + 1) {
            let a_end = a.offset + a.new.len();
            let b_end = b.offset + b.new.len();
            if a.offset < b_end && b.offset < a_end {
                notes.push(format!(
                    "edits at 0x{:X} and 0x{:X} overlap - the later one wins",
                    a.offset, b.offset
                ));
            }
        }
    }
    notes
}

pub use apply::apply_ops;

/// Mutates `data` in place by applying the report's diffs (offsets are
/// absolute, so application order does not matter).
pub fn apply_diffs(data: &mut [u8], diffs: &[Diff]) {
    for d in diffs {
        data[d.offset..d.offset + d.new.len()].copy_from_slice(&d.new);
    }
}

/// Refuses the write if any byte in the protected layout ranges
/// changed outside the reported diffs (i.e. not as a deliberate,
/// validated edit). Catches `--hex` or locator bugs that would
/// silently retarget a table or corrupt a structure header while
/// keeping the image parseable.
pub fn verify_structural(orig: &[u8], patched: &[u8], diffs: &[Diff]) -> Result<()> {
    let r = Reader::new(patched);
    let map = map::map_rom(patched)?;
    let mut violations = Vec::new();
    'ranges: for (start, end) in map::layout_ranges(&map, &r) {
        for i in start..end {
            if orig.get(i) == patched.get(i) {
                continue;
            }
            let edited = diffs
                .iter()
                .any(|d| i >= d.offset && i < d.offset + d.new.len());
            if !edited {
                violations.push(i);
                if violations.len() >= 5 {
                    break 'ranges;
                }
            }
        }
    }
    if !violations.is_empty() {
        let list = violations
            .iter()
            .map(|o| format!("0x{o:X}"))
            .collect::<Vec<_>>()
            .join(", ");
        anyhow::bail!(
            "structural drift: protected layout bytes changed at {list} - refusing to write. \
             Only --hex into understood data areas is allowed; structure headers, the boot \
             area, the master tables and the data structures must stay untouched"
        );
    }
    Ok(())
}

/// Post-patch verification: every parser must still succeed and every
/// command table must still disassemble to EOT. Returns `Err` on any
/// structural failure (refuse to write), `Ok(validate warnings)` when
/// the image parses cleanly but has non-fatal sanity warnings.
pub fn verify(data: &[u8]) -> Result<Vec<String>> {
    let mut w = Vec::new();
    let rom = crate::rom::parse_bytes(data, "patched.rom")
        .map_err(|e| anyhow::anyhow!("post-patch re-parse failed: {e:#}"))?;
    w.extend(rom.warnings.iter().cloned());
    let r = Reader::new(data);
    let mct = r.u16(rom.header.atom_header_offset + 0x1e)? as usize;
    crate::rom::disasm::disasm_command_tables(&r, mct, None, None)
        .map_err(|e| anyhow::anyhow!("post-patch disasm sweep failed: {e:#}"))?;
    Ok(w)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rom::header;
    use crate::rom::locate;

    fn sample() -> std::path::PathBuf {
        std::path::PathBuf::from("samples/BIOS/RX570/Sapphire.RX570.4096.170317_2.rom")
    }

    #[test]
    fn roundtrip_every_op() {
        if !sample().exists() {
            eprintln!("sample ROMs not present; skipping patch roundtrip");
            return;
        }
        let orig = std::fs::read(sample()).unwrap();
        let rom = crate::rom::parse_rom(&sample()).unwrap();

        let clock = rom.vram.straps.first().unwrap().clock_mhz.round() as u32;
        let new_clock = clock + 25;
        let reg0_new = rom.vram.straps.first().unwrap().values[0] ^ 0xABCD;
        let ops = vec![
            PatchOp::SetStrap {
                clock_mhz: clock,
                reg: 0,
                value: reg0_new,
            },
            PatchOp::RetagStrap {
                clock_mhz: clock,
                new_clock_mhz: new_clock,
            },
            PatchOp::PpSclk {
                level: 0,
                mhz: 1000,
            },
            PatchOp::PpMclk {
                level: 0,
                mhz: 1750,
            },
            PatchOp::PpVddc { index: 0, mv: 800 },
            PatchOp::PpTdp { watts: 135 },
        ];
        let report = apply_ops(&orig, &ops, false).expect("ops accepted");
        let mut patched = orig.clone();
        apply_diffs(&mut patched, &report.diffs);
        assert_eq!(patched.len(), orig.len());

        // Re-parse: target fields changed.
        let parsed = crate::rom::parse_bytes(&patched, "patched.rom").unwrap();
        let s = parsed.vram.straps.first().unwrap();
        assert_eq!(s.clock_mhz, new_clock as f64, "retag applied");
        assert_eq!(s.values[0], reg0_new, "strap reg applied");
        assert_eq!(parsed.powerplay.sclk_table[0].sclk_mhz, 1000.0);
        assert_eq!(parsed.powerplay.mclk_table[0].mclk_mhz, 1750.0);
        assert_eq!(parsed.powerplay.vddc_lut[0].vdd_mv, 800);
        assert_eq!(parsed.powerplay.powertune.as_ref().unwrap().tdp_w, 135);

        // Outside the edited ranges the image is byte-identical.
        for d in &report.diffs {
            for i in 0..d.new.len() {
                patched[d.offset + i] = orig[d.offset + i];
            }
        }
        assert_eq!(patched, orig, "only the edited bytes changed");

        // Checksum recompute: fixed image validates.
        fix_checksum(&mut patched).expect("fix works");
        let sum_after: u8 = patched[..rom.header.checksum_checked_bytes]
            .iter()
            .fold(0, |a, b| a.wrapping_add(*b));
        assert_eq!(sum_after, 0, "checksum wraps to 0");
        assert!(
            crate::rom::parse_bytes(&patched, "patched.rom")
                .unwrap()
                .header
                .checksum_valid
        );
    }

    #[test]
    fn refuses_invalid_checksum_input() {
        if !sample().exists() {
            return;
        }
        let mut data = std::fs::read(sample()).unwrap();
        data[0x100] ^= 0x01;
        let err = apply_ops(&data, &[PatchOp::PpTdp { watts: 100 }], false)
            .expect_err("invalid checksum refused");
        assert!(err.to_string().contains("checksum"), "got: {err}");
    }

    /// `--fix-checksum` repair path: a corrupted image must be
    /// repairable and then patchable (the input gate is skipped when
    /// the caller signals the repair was performed).
    #[test]
    fn fix_checksum_repair_path() {
        if !sample().exists() {
            return;
        }
        let mut data = std::fs::read(sample()).unwrap();
        data[0x100] ^= 0x40; // break the checksum

        let mut fixed = data.clone();
        fix_checksum(&mut fixed)
            .expect("repair works")
            .expect("was fixed");

        let report = apply_ops(&fixed, &[PatchOp::PpTdp { watts: 100 }], true)
            .expect("ops accepted on repaired image");
        apply_diffs(&mut fixed, &report.diffs);
        fix_checksum(&mut fixed).expect("post-fix works");
        let parsed = crate::rom::parse_bytes(&fixed, "fixed.rom").unwrap();
        assert!(
            parsed.header.checksum_valid,
            "repaired image is checksum-valid"
        );
        assert_eq!(parsed.powerplay.powertune.as_ref().unwrap().tdp_w, 100);
    }

    #[test]
    fn refuses_absurd_clocks() {
        if !sample().exists() {
            return;
        }
        let orig = std::fs::read(sample()).unwrap();
        let cases: &[PatchOp] = &[
            PatchOp::PpSclk {
                level: 0,
                mhz: 42_949_673,
            },
            PatchOp::PpMclk {
                level: 0,
                mhz: u32::MAX,
            },
            PatchOp::RetagStrap {
                clock_mhz: 1750,
                new_clock_mhz: u32::MAX,
            },
            PatchOp::SetStrap {
                clock_mhz: u32::MAX,
                reg: 0,
                value: 0,
            },
        ];
        for op in cases {
            let err = apply_ops(&orig, std::slice::from_ref(op), false)
                .expect_err("absurd clock refused");
            assert!(format!("{err:#}").contains("implausible"), "got: {err:#}");
        }
        // A normal value still works.
        apply_ops(
            &orig,
            &[PatchOp::PpSclk {
                level: 0,
                mhz: 1000,
            }],
            false,
        )
        .expect("sane clock accepted");
    }

    #[test]
    fn refuses_hex_in_protected_layout() {
        if !sample().exists() {
            return;
        }
        let data = std::fs::read(sample()).unwrap();
        let r = Reader::new(&data);
        let rom = crate::rom::parse_rom(&sample()).unwrap();
        let mdt = r.u16(rom.header.atom_header_offset + 0x20).unwrap() as usize;
        let pp_off = crate::rom::header::master_table_offset(&r, mdt, "PowerPlayInfo").unwrap();
        let pcir_off = crate::rom::pci::walk_pci_images(&r).unwrap()[0].pcir_offset;
        let cases = [
            (0x48usize, "entry point"),
            (rom.header.atom_header_offset, "ATOM header"),
            (mdt, "master data table"),
            (rom.header.master_cmd_table_offset, "command table list"),
            (pp_off, "PowerPlay header"),
            (pcir_off, "PCI data structure"),
        ];
        for (off, what) in cases {
            let err = apply_ops(
                &data,
                &[PatchOp::Hex {
                    offset: off,
                    bytes: vec![0x00],
                }],
                false,
            )
            .expect_err(what);
            assert!(
                format!("{err:#}").contains("refusing --hex"),
                "{what}: got {err}"
            );
        }
        // A hex write into strap DATA (not the index table) stays allowed.
        let vram_off = crate::rom::header::master_table_offset(&r, mdt, "VRAM_Info").unwrap();
        let strap_off = locate::strap_value(&r, vram_off, 0, 0).expect("strap value locator");
        let cur = r.u32(strap_off).unwrap();
        let report = apply_ops(
            &data,
            &[PatchOp::Hex {
                offset: strap_off,
                bytes: (cur ^ 1).to_le_bytes().to_vec(),
            }],
            false,
        )
        .expect("strap data hex allowed");
        assert!(
            report.warnings.iter().any(|w| w.contains("overlaps")),
            "expected an overlap warning, got: {:?}",
            report.warnings
        );
    }

    #[test]
    fn verify_structural_rejects_drift() {
        if !sample().exists() {
            return;
        }
        let orig = std::fs::read(sample()).unwrap();
        let report = apply_ops(
            &orig,
            &[PatchOp::PpSclk {
                level: 0,
                mhz: 1000,
            }],
            false,
        )
        .unwrap();
        let mut patched = orig.clone();
        apply_diffs(&mut patched, &report.diffs);
        verify_structural(&orig, &patched, &report.diffs).expect("no drift for a legit op");

        // Corrupting a layout byte (PP table revision) must be caught.
        let r = Reader::new(&orig);
        let rom = crate::rom::parse_rom(&sample()).unwrap();
        let mdt = r.u16(rom.header.atom_header_offset + 0x20).unwrap() as usize;
        let pp_off = crate::rom::header::master_table_offset(&r, mdt, "PowerPlayInfo").unwrap();
        patched[pp_off + 4] ^= 1;
        let err = verify_structural(&orig, &patched, &report.diffs)
            .expect_err("structural drift detected");
        assert!(err.to_string().contains("structural drift"), "got: {err}");
    }

    #[test]
    fn refuses_vddc_above_die_max() {
        let msi = std::path::PathBuf::from("samples/BIOS/RX550/MSI.RX550.2048.170509.rom");
        if !msi.exists() {
            return;
        }
        let data = std::fs::read(&msi).unwrap();
        let rom = crate::rom::parse_rom(&msi).unwrap();
        let Some(die_max) = rom.profiling.map(|p| p.max_vddc_mv / 100) else {
            eprintln!("MSI RX550 has no ASIC_ProfilingInfo; skipping die-max test");
            return;
        };
        assert!(
            die_max > 0 && die_max < 1500,
            "unexpected die max {die_max}"
        );
        let err = apply_ops(
            &data,
            &[PatchOp::PpVddc {
                index: 0,
                mv: die_max as u16 + 1,
            }],
            false,
        )
        .expect_err("above die max refused");
        assert!(format!("{err:#}").contains("die maximum"), "got: {err:#}");
        apply_ops(
            &data,
            &[PatchOp::PpVddc {
                index: 0,
                mv: die_max as u16 - 50,
            }],
            false,
        )
        .expect("under die max accepted");
    }

    /// `--set-strap` edits a register value without touching the clock,
    /// so it must be allowed even when the strap's stock clock sits
    /// above the ROM's hard limit (Lenovo/Medion ship that way).
    /// Changing the clock (retag / DPM) above the limit stays refused.
    #[test]
    fn set_strap_allowed_above_hard_limit() {
        let lenovo = std::path::PathBuf::from("samples/BIOS/RX560/Lenovo.RX560.4096.170822.rom");
        if !lenovo.exists() {
            return;
        }
        let data = std::fs::read(&lenovo).unwrap();
        let rom = crate::rom::parse_rom(&lenovo).unwrap();
        let Some(limit) = rom.powerplay.hard_limits.first().map(|h| h.mclk_limit_mhz) else {
            return;
        };
        let Some(strap) = rom.vram.straps.iter().find(|s| s.clock_mhz > limit) else {
            return;
        };
        let clock = strap.clock_mhz.round() as u32;
        apply_ops(
            &data,
            &[PatchOp::SetStrap {
                clock_mhz: clock,
                reg: 0,
                value: strap.values[0] ^ 0xABCD,
            }],
            false,
        )
        .expect("register edit at the existing clock is allowed above the hard limit");
        let err = apply_ops(
            &data,
            &[PatchOp::RetagStrap {
                clock_mhz: clock,
                new_clock_mhz: clock + 5,
            }],
            false,
        )
        .expect_err("retag above hard limit refused");
        assert!(format!("{err:#}").contains("hard limit"), "got: {err:#}");
    }

    #[test]
    fn refuses_noop_and_over_limit() {
        if !sample().exists() {
            return;
        }
        let orig = std::fs::read(sample()).unwrap();
        let rom = crate::rom::parse_rom(&sample()).unwrap();
        let tdp = rom.powerplay.powertune.as_ref().unwrap().tdp_w;
        let err =
            apply_ops(&orig, &[PatchOp::PpTdp { watts: tdp }], false).expect_err("no-op refused");
        assert!(
            err.to_string().contains("no-op") || err.to_string().contains("rejected"),
            "got: {err}"
        );

        if let Some(hl) = rom.powerplay.hard_limits.first() {
            let limit = hl.mclk_limit_mhz.round() as u32 + 1;
            let err = apply_ops(
                &orig,
                &[PatchOp::PpMclk {
                    level: 0,
                    mhz: limit,
                }],
                false,
            )
            .expect_err("over hard limit refused");
            assert!(format!("{err:#}").contains("hard limit"), "got: {err:#}");
        } else {
            // The Sapphire sample has no hard limit table; exercise the
            // guard against the Lenovo RX560, which does (MCLK 625).
            let lenovo =
                std::path::PathBuf::from("samples/BIOS/RX560/Lenovo.RX560.4096.170822.rom");
            if lenovo.exists() {
                let data = std::fs::read(&lenovo).unwrap();
                let err = apply_ops(
                    &data,
                    &[PatchOp::PpMclk {
                        level: 0,
                        mhz: 1000,
                    }],
                    false,
                )
                .expect_err("over hard limit refused");
                assert!(format!("{err:#}").contains("hard limit"), "got: {err:#}");
            }
        }
    }

    #[test]
    fn fix_checksum_uses_padding() {
        if !sample().exists() {
            return;
        }
        let mut data = std::fs::read(sample()).unwrap();
        let rom = crate::rom::parse_rom(&sample()).unwrap();
        let last = rom.header.checksum_checked_bytes - 1;
        assert_eq!(data[last], 0xFF, "last declared byte is padding");
        data[0x200] ^= 0x40; // break the checksum
        let diff = fix_checksum(&mut data).unwrap().expect("fixed");
        assert_eq!(diff.offset, last, "fix lands on the last declared byte");
    }

    /// A2: the BIOS data area (build date at 0x50, vendor block) sits
    /// between the boot area and the ATOM header; `--hex` there must be
    /// refused like any other layout byte.
    #[test]
    fn refuses_hex_in_bios_data_area() {
        if !sample().exists() {
            return;
        }
        let data = std::fs::read(sample()).unwrap();
        for off in [0x50usize, 0x60, 0x90] {
            let err = apply_ops(
                &data,
                &[PatchOp::Hex {
                    offset: off,
                    bytes: vec![0x00],
                }],
                false,
            )
            .expect_err("bios data area refused");
            assert!(
                format!("{err:#}").contains("refusing --hex"),
                "0x{off:X}: got {err:#}"
            );
        }
        // Right before the ATOM header is also protected.
        let rom = crate::rom::parse_rom(&sample()).unwrap();
        let err = apply_ops(
            &data,
            &[PatchOp::Hex {
                offset: rom.header.atom_header_offset - 1,
                bytes: vec![0x00],
            }],
            false,
        )
        .expect_err("pre-header byte refused");
        assert!(format!("{err:#}").contains("refusing --hex"), "{err:#}");
    }

    /// A1: `--hex` into command table bytecode must produce the
    /// structure-overlap warning (it changes firmware behavior).
    #[test]
    fn hex_into_command_table_warns() {
        if !sample().exists() {
            return;
        }
        let data = std::fs::read(sample()).unwrap();
        let r = Reader::new(&data);
        let rom = crate::rom::parse_rom(&sample()).unwrap();
        let mct = rom.header.master_cmd_table_offset;
        let mut target = None;
        for idx in 0..crate::rom::header::COMMAND_TABLE_NAMES.len() {
            if let Ok(off) = r.u16(mct + 4 + idx * 2)
                && off != 0
            {
                target = Some(off as usize + 8); // inside the code region
                break;
            }
        }
        let Some(target) = target else {
            panic!("no command table in sample");
        };
        let cur = r.u8(target).unwrap();
        let report = apply_ops(
            &data,
            &[PatchOp::Hex {
                offset: target,
                bytes: vec![cur ^ 1],
            }],
            false,
        )
        .expect("command table data is editable (warned, not refused)");
        assert!(
            report.warnings.iter().any(|w| w.contains("overlaps")),
            "expected a structure overlap warning, got: {:?}",
            report.warnings
        );
    }

    /// M1: a strap register index past the strap block's own slots must
    /// be refused (it would write into the next strap's clock field).
    #[test]
    fn refuses_strap_reg_outside_block() {
        if !sample().exists() {
            return;
        }
        let data = std::fs::read(sample()).unwrap();
        let rom = crate::rom::parse_rom(&sample()).unwrap();
        let Some(strap) = rom.vram.straps.first() else {
            return;
        };
        let clock = strap.clock_mhz.round() as u32;
        let r = Reader::new(&data);
        let mct = r.u16(rom.header.atom_header_offset + 0x20).unwrap() as usize;
        let vram_off = header::master_table_offset(&r, mct, "VRAM_Info").unwrap();
        let (_, block_size) = locate::strap_region(&r, vram_off).expect("strap region");
        let n_vals = block_size / 4 - 1;
        let err = apply_ops(
            &data,
            &[PatchOp::SetStrap {
                clock_mhz: clock,
                reg: n_vals,
                value: 0x1234_5678,
            }],
            false,
        )
        .expect_err("register outside the strap block refused");
        assert!(
            format!("{err:#}").contains("outside the strap block"),
            "got: {err:#}"
        );
    }

    /// M2: retagging a strap above the highest strap clock the ROM
    /// ships is refused (the MC only trains those); retagging to a
    /// clock with no MCLK DPM level warns.
    #[test]
    fn retag_above_trained_clock_refused_and_dpm_mismatch_warns() {
        if !sample().exists() {
            return;
        }
        let data = std::fs::read(sample()).unwrap();
        let rom = crate::rom::parse_rom(&sample()).unwrap();
        let Some(strap) = rom.vram.straps.first() else {
            return;
        };
        let clock = strap.clock_mhz.round() as u32;
        let r = Reader::new(&data);
        let mdb = r.u16(rom.header.atom_header_offset + 0x20).unwrap() as usize;
        let vram_off = header::master_table_offset(&r, mdb, "VRAM_Info").unwrap();
        let max_clock = super::limits::max_strap_clock(&r, vram_off).expect("max strap clock");
        let err = apply_ops(
            &data,
            &[PatchOp::RetagStrap {
                clock_mhz: clock,
                new_clock_mhz: max_clock + 100,
            }],
            false,
        )
        .expect_err("retag above trained clock refused");
        assert!(
            format!("{err:#}").contains("only trains straps up to"),
            "got: {err:#}"
        );

        // Retag to a clock that exists in the straps but not in the
        // MCLK DPM table: warned, not refused.
        let mclk_clocks: Vec<u32> = rom
            .powerplay
            .mclk_table
            .iter()
            .map(|e| e.mclk_mhz.round() as u32)
            .collect();
        let Some(no_dpm) = rom
            .vram
            .straps
            .iter()
            .map(|s| s.clock_mhz.round() as u32)
            .find(|c| !mclk_clocks.contains(c) && *c != clock)
        else {
            eprintln!("no usable strap clock without an MCLK DPM match; skipping warn check");
            return;
        };
        let report = apply_ops(
            &data,
            &[PatchOp::RetagStrap {
                clock_mhz: clock,
                new_clock_mhz: no_dpm,
            }],
            false,
        )
        .expect("retag without DPM match is allowed (warned)");
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.contains("no MCLK DPM level")),
            "expected a DPM mismatch warning, got: {:?}",
            report.warnings
        );
    }

    /// L2: VDDC above the ROM's own hard limit warns but applies (the
    /// SMC clamps); only the die maximum refuses.
    #[test]
    fn vddc_above_hard_limit_warns_not_refuses() {
        let lenovo = std::path::PathBuf::from("samples/BIOS/RX560/Lenovo.RX560.4096.170822.rom");
        if !lenovo.exists() {
            return;
        }
        let data = std::fs::read(&lenovo).unwrap();
        let rom = crate::rom::parse_rom(&lenovo).unwrap();
        let Some(limit) = rom.powerplay.hard_limits.first().map(|h| h.vddc_limit_mv) else {
            eprintln!("Lenovo sample has no VDDC hard limit; skipping");
            return;
        };
        if !(1..2000).contains(&limit) {
            eprintln!("VDDC hard limit {limit} mV looks invalid; skipping");
            return;
        }
        let report = apply_ops(
            &data,
            &[PatchOp::PpVddc {
                index: 0,
                mv: limit + 10,
            }],
            false,
        )
        .expect("above hard limit but under die max: warned, not refused");
        assert!(
            report.warnings.iter().any(|w| w.contains("hard limit")),
            "expected a hard limit warning, got: {:?}",
            report.warnings
        );
    }

    /// L5: implausible-but-encodable clocks warn instead of passing
    /// silently.
    #[test]
    fn implausible_clock_warns() {
        if !sample().exists() {
            return;
        }
        let data = std::fs::read(sample()).unwrap();
        let report = apply_ops(
            &data,
            &[PatchOp::PpSclk {
                level: 0,
                mhz: 3000,
            }],
            false,
        )
        .expect("above typical range is allowed (warned)");
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.contains("outside the usual 100-2500")),
            "expected a range warning, got: {:?}",
            report.warnings
        );
        let report = apply_ops(
            &data,
            &[PatchOp::PpMclk {
                level: 0,
                mhz: 3100,
            }],
            false,
        )
        .expect("above typical range is allowed (warned)");
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.contains("outside the usual 100-3000")),
            "expected a range warning, got: {:?}",
            report.warnings
        );
    }

    /// L3: overlapping edits are flagged (the later one wins during
    /// `apply_diffs`).
    #[test]
    fn overlapping_diffs_are_reported() {
        let a = Diff {
            offset: 0x100,
            old: vec![0; 4],
            new: vec![1; 4],
            label: "a".into(),
        };
        let b = Diff {
            offset: 0x102,
            old: vec![0; 4],
            new: vec![2; 4],
            label: "b".into(),
        };
        let notes = overlapping_diffs(&[a, b]);
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert!(notes[0].contains("overlap"), "{notes:?}");
        assert!(
            overlapping_diffs(&[Diff {
                offset: 0x100,
                old: vec![0; 4],
                new: vec![1; 4],
                label: "a".into(),
            }])
            .is_empty()
        );
    }
}
