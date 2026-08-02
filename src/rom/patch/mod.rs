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
