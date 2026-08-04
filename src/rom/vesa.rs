use anyhow::Result;

use super::reader::Reader;
use super::types::{VesaInfo, VesaMode};

/// Parses the StandardVESA_Timing data table (`ATOM_STANDARD_VESA_TIMING`,
/// atombios.h line 7412): the list of native VESA modes the BIOS can
/// drive without a display driver. Each entry is an `ATOM_DTD_FORMAT`
/// (28 bytes): pixel clock in 10 kHz units, active/blanking times,
/// sync offsets and widths, mode misc flags and the internal mode
/// number. All-zero entries are skipped (empty slots in the table).
pub fn parse_vesa_timing(r: &Reader, off: usize) -> Result<VesaInfo> {
    let (struct_size, fmt_rev, cont_rev) = r.table_header(off)?;

    let avail = (struct_size as usize).saturating_sub(4) / 28;
    let mut modes = Vec::with_capacity(avail);
    for i in 0..avail {
        let p = off + 4 + i * 28;
        let pix_10khz = r.u16(p)?;
        let h_active = r.u16(p + 2)?;
        let h_blanking = r.u16(p + 4)?;
        let v_active = r.u16(p + 6)?;
        let v_blanking = r.u16(p + 8)?;
        let h_sync_offset = r.u16(p + 10)?;
        let h_sync_width = r.u16(p + 12)?;
        let v_sync_offset = r.u16(p + 14)?;
        let v_sync_width = r.u16(p + 16)?;
        let misc = r.u16(p + 24)?;
        let internal_mode_number = r.u8(p + 26)?;

        if pix_10khz == 0 && h_active == 0 && v_active == 0 {
            continue;
        }

        let h_total = h_active as u32 + h_blanking as u32;
        let v_total = v_active as u32 + v_blanking as u32;
        let refresh_rate_hz = if h_total > 0 && v_total > 0 {
            pix_10khz as f64 * 10_000.0 / (h_total as f64 * v_total as f64)
        } else {
            0.0
        };

        // ATOM_MODE_MISC_INFO (little-endian): bit1 = HSyncPolarity
        // (0=active high), bit2 = VSyncPolarity (0=active high).
        let sync_polarity = format!(
            "{}HSync/{}VSync",
            if misc & (1 << 1) == 0 { '+' } else { '-' },
            if misc & (1 << 2) == 0 { '+' } else { '-' },
        );

        modes.push(VesaMode {
            index: i,
            pixel_clock_mhz: pix_10khz as f64 / 100.0,
            h_active,
            h_blanking,
            v_active,
            v_blanking,
            h_sync_offset,
            h_sync_width,
            v_sync_offset,
            v_sync_width,
            refresh_rate_hz,
            sync_polarity,
            internal_mode_number,
        });
    }

    Ok(VesaInfo {
        struct_size,
        fmt_rev,
        cont_rev,
        modes,
    })
}
