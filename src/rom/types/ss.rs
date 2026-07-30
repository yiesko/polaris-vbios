use serde::Serialize;

/// `ATOM_ASIC_SS_ASSIGNMENT_V3` - one spread-spectrum entry: the clock
/// branch (memory/engine/DP/...) it applies to, the spread percentage
/// and the modulation rate.
#[derive(Debug, Clone, Serialize)]
pub struct SsEntry {
    pub index: usize,
    /// ulTargetClockRange in 10 kHz; `0x00FFFFFF` means "not limited
    /// to a clock range" (applies to all clocks of that branch).
    pub target_clock_range_khz10: u32,
    /// usSpreadSpectrumPercentage in 0.01% (or 0.001% when mode bit 4
    /// is set). `None` when the target is the all-clock sentinel.
    pub spread_pct: f64,
    /// usSpreadRateIn10Hz - modulation frequency.
    pub spread_rate_hz10: u16,
    /// ucClockIndication - which clock source needs SS (see
    /// ASIC_INTERNAL_*_SS defines in atombios.h).
    pub clock_indication: u8,
    /// ucSpreadSpectrumMode: bit0 = centre (1) vs down (0) spread,
    /// bit1 = external (1) vs internal (0) SS.
    pub spread_mode: u8,
}

/// `ATOM_ASIC_INTERNAL_SS_INFO_V3` - spread spectrum configuration of
/// the internal clock generators.
#[derive(Debug, Clone, Serialize)]
pub struct SsInfo {
    pub struct_size: u16,
    pub fmt_rev: u8,
    pub cont_rev: u8,
    pub entries: Vec<SsEntry>,
}
