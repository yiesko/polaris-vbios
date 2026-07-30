use anyhow::Result;

use crate::rom::header;
use crate::rom::locate;
use crate::rom::reader::Reader;

/// One entry of the ROM's PowerPlay hard limit table (limits the
/// firmware itself refuses to exceed). All values in human units.
pub(super) struct HardLimitRec {
    pub(super) sclk_mhz: f64,
    pub(super) mclk_mhz: f64,
    pub(super) vddc_mv: u16,
}

pub(super) fn read_hard_limits(r: &Reader, pp_off: Option<usize>) -> Vec<HardLimitRec> {
    let Some(pp_off) = pp_off else {
        return Vec::new();
    };
    let Some(hl_off) = locate::powerplay_hard_limits(r, pp_off) else {
        return Vec::new();
    };
    let Ok(n) = r.u8(hl_off + 1) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut p = hl_off + 2;
    for _ in 0..n {
        match (r.u32(p), r.u32(p + 4), r.u16(p + 8)) {
            (Ok(sclk), Ok(mclk), Ok(vddc)) => {
                out.push(HardLimitRec {
                    sclk_mhz: sclk as f64 / 100.0,
                    mclk_mhz: mclk as f64 / 100.0,
                    vddc_mv: vddc,
                });
            }
            _ => break,
        }
        p += 14;
    }
    out
}

pub(super) fn max_limit(hard_limits: &[HardLimitRec], f: fn(&HardLimitRec) -> f64) -> f64 {
    hard_limits.iter().map(f).fold(0.0, f64::max)
}

/// Highest clock (MHz) the ROM's memory straps are tagged with; the
/// firmware's memory controller only trains those straps, so retagging
/// above it would program an untrained clock.
pub(super) fn max_strap_clock(r: &Reader, vram_off: usize) -> Option<u32> {
    let (data_start, block_size) = locate::strap_region(r, vram_off)?;
    let n = strap_count(r, data_start, block_size);
    (0..n)
        .filter_map(|i| r.u32(data_start + i * block_size).ok())
        .map(|raw| (raw & 0xFF_FFFF) / 100)
        .max()
}

/// Refuses a clock above the ROM's declared hard limit of that kind.
/// A limit of 0.0 means "no hard limit table" - no guard.
pub(super) fn guard_clock(
    hard_limits: &[HardLimitRec],
    value_mhz: f64,
    limit_of: fn(&HardLimitRec) -> f64,
    kind: &str,
) -> Result<()> {
    let limit = max_limit(hard_limits, limit_of);
    if limit > 0.0 && value_mhz > limit {
        anyhow::bail!(
            "{kind} {value_mhz:.0} MHz exceeds the hard limit {limit:.0} MHz declared by this \
             ROM's PowerPlay table"
        );
    }
    Ok(())
}

/// Validates a clock in MHz and converts it to the 100x centi-MHz
/// units used in the tables, without wrapping. Clocks above 65535 MHz
/// are rejected outright - they cannot be real and the u32 multiply
/// would otherwise wrap around silently in release builds.
pub(super) fn centi_mhz(mhz: u32) -> Result<u32> {
    let c = mhz
        .checked_mul(100)
        .filter(|c| *c <= 65_535 * 100)
        .ok_or_else(|| anyhow::anyhow!("clock {mhz} MHz is implausible (max 65535 MHz)"))?;
    Ok(c)
}

/// Number of strap blocks currently in the ROM (parses the terminator
/// the same way `vram.rs::parse_memory_straps` does).
pub(super) fn strap_count(r: &Reader, data_start: usize, block_size: usize) -> usize {
    let mut n = 0;
    let mut p = data_start;
    while p + block_size <= r.len() && n < 64 {
        match r.u32(p) {
            Ok(raw) if raw != 0 || n == 0 => {
                p += block_size;
                n += 1;
            }
            _ => break,
        }
    }
    n
}

pub(super) fn find_strap(r: &Reader, vram_off: usize, clock_mhz: u32) -> Result<usize> {
    let (data_start, block_size) = locate::strap_region(r, vram_off)
        .ok_or_else(|| anyhow::anyhow!("ROM has no memory strap table"))?;
    let n = strap_count(r, data_start, block_size);
    let want = centi_mhz(clock_mhz)?;
    for i in 0..n {
        let raw = r.u32(data_start + i * block_size)?;
        if raw & 0xFF_FFFF == want {
            return Ok(i);
        }
    }
    let clocks = (0..n)
        .filter_map(|i| r.u32(data_start + i * block_size).ok())
        .map(|raw| format!("{}", (raw & 0xFF_FFFF) / 100))
        .collect::<Vec<_>>()
        .join(", ");
    anyhow::bail!("no strap with clock {clock_mhz} MHz found (available: {clocks})")
}

/// Number of registers each strap block's index table lists.
pub(super) fn n_regs(r: &Reader, vram_off: usize) -> usize {
    let patch_off = match r.u16(vram_off + 6) {
        Ok(v) => v as usize,
        Err(_) => return 0,
    };
    match r.u16(vram_off + patch_off) {
        Ok(sz) => sz as usize / 3,
        Err(_) => 0,
    }
}

/// Die VDDC maximum in mV from `ASIC_ProfilingInfo` (0 when unknown).
/// The stored value is in 0.01 V units (same conversion as validate.rs).
pub(super) fn die_max_mv(r: &Reader, mdt: usize) -> Option<u32> {
    let off = header::master_table_offset(r, mdt, "ASIC_ProfilingInfo").ok()?;
    let info = crate::rom::profiling::parse_profiling_info(r, off).ok()?;
    (info.max_vddc_mv > 0).then_some(info.max_vddc_mv / 100)
}
