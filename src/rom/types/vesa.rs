use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct VesaMode {
    pub index: usize,
    /// usPixClk in 10 kHz units.
    pub pixel_clock_mhz: f64,
    pub h_active: u16,
    pub h_blanking: u16,
    pub v_active: u16,
    pub v_blanking: u16,
    pub h_sync_offset: u16,
    pub h_sync_width: u16,
    pub v_sync_offset: u16,
    pub v_sync_width: u16,
    /// Derived from pixel clock and total H/V - the mode's refresh rate.
    pub refresh_rate_hz: f64,
    /// Sync polarity from the mode misc bits, e.g. "+HSync/+VSync".
    pub sync_polarity: String,
    pub internal_mode_number: u8,
}

/// `ATOM_STANDARD_VESA_TIMING` - the list of native VESA modes the
/// BIOS supports (StandardVESA_Timing data table, atombios.h line 7412).
#[derive(Debug, Clone, Serialize)]
pub struct VesaInfo {
    pub struct_size: u16,
    pub fmt_rev: u8,
    pub cont_rev: u8,
    pub modes: Vec<VesaMode>,
}
