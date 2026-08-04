use anyhow::Result;

use super::reader::Reader;
use super::types::{SsEntry, SsInfo};

/// Spread spectrum clock indication IDs (atombios.h line 6641).
pub fn ss_clock_name(ind: u8) -> &'static str {
    match ind {
        1 => "Memory",
        2 => "Engine",
        3 => "UVD",
        4 => "TMDS",
        5 => "HDMI",
        6 => "LVDS",
        7 => "DP",
        8 => "DCPLL",
        9 => "External DP clock",
        10 => "VCE",
        11 => "GPUPLL",
        _ => "Unknown",
    }
}

/// Parses `ATOM_ASIC_INTERNAL_SS_INFO_V3` (atombios.h line 6701):
/// variable-length list of `ATOM_ASIC_SS_ASSIGNMENT_V3` (12 bytes each)
/// describing the spread spectrum settings of the internal clock
/// generators. `ulTargetClockRange` is in 10 kHz units; the value
/// 0x00FFFFFF means "applies to all clocks of that branch".
pub fn parse_ss_info(r: &Reader, off: usize) -> Result<SsInfo> {
    let (struct_size, fmt_rev, cont_rev) = r.table_header(off)?;
    let entry_bytes = 12usize;
    let count = struct_size.saturating_sub(4) as usize / entry_bytes;

    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        let e = off + 4 + i * entry_bytes;
        let spread_pct_raw = r.u16(e + 4)?;
        let mode = r.u8(e + 9)?;
        // Bit 4 of ucSpreadSpectrumMode: percentage is in 0.001%
        // instead of 0.01%.
        let spread_pct = if mode & 0x10 != 0 {
            spread_pct_raw as f64 / 1000.0
        } else {
            spread_pct_raw as f64 / 100.0
        };
        entries.push(SsEntry {
            index: i,
            target_clock_range_khz10: r.u32(e)?,
            spread_pct,
            spread_rate_hz10: r.u16(e + 6)?,
            clock_indication: r.u8(e + 8)?,
            spread_mode: mode,
        });
    }

    Ok(SsInfo {
        struct_size,
        fmt_rev,
        cont_rev,
        entries,
    })
}
