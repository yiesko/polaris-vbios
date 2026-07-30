use anyhow::{Result, bail};

use super::types::Diff;
use crate::rom::reader::Reader;

/// (declared bytes, wrapping u8 sum over them).
pub fn checksum_state(data: &[u8]) -> Result<(usize, u8)> {
    let r = Reader::new(data);
    let declared = r.u8(2)? as usize * 512;
    let checked = declared.min(data.len());
    if checked == 0 {
        bail!("declared ROM region is empty");
    }
    let sum = data[..checked]
        .iter()
        .fold(0u8, |acc, b| acc.wrapping_add(*b));
    Ok((checked, sum))
}

/// Recomputes the legacy checksum: adjusts the LAST byte of the declared
/// region (0xFF padding in all surveyed ROMs) so the sum wraps to 0x00.
/// Returns None when the checksum is already valid.
pub fn fix_checksum(data: &mut [u8]) -> Result<Option<Diff>> {
    let (checked, sum) = checksum_state(data)?;
    if sum == 0 {
        return Ok(None);
    }
    let last = checked - 1;
    let target = if data[last] == 0xFF {
        last
    } else {
        // Fallback: scan back up to 16 bytes for padding; refuse if
        // none is found (can't recompute safely).
        let start = last.saturating_sub(16);
        match data[start..=last].iter().rposition(|&b| b == 0xFF) {
            Some(rel) => start + rel,
            None => {
                bail!(
                    "cannot recompute checksum: last byte of the declared region (0x{last:X}) is \
                     not padding and no 0xFF padding found in the 16 bytes before it"
                );
            }
        }
    };
    let old_byte = data[target];
    // new = old - sum  =>  sum + (new - old) ≡ 0 (mod 256)
    let new_byte = old_byte.wrapping_sub(sum);
    if new_byte == old_byte {
        bail!("internal error: checksum fix produced a no-op");
    }
    data[target] = new_byte;
    Ok(Some(Diff {
        offset: target,
        old: vec![old_byte],
        new: vec![new_byte],
        label: "legacy checksum byte".to_string(),
    }))
}
