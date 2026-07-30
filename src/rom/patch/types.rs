/// One edit requested on the command line. All offsets/values are
/// validated by [`apply_ops`] before anything is written.
#[derive(Debug, Clone)]
pub enum PatchOp {
    /// Set strap register `reg` (position in the strap reg index table)
    /// of the strap whose clock is `clock_mhz`.
    SetStrap {
        clock_mhz: u32,
        reg: usize,
        value: u32,
    },
    /// Set the same MC register (identified by its absolute offset in
    /// the strap reg index table) in every strap block.
    SetStrapReg { reg_offset: u32, value: u32 },
    /// Change the clock a strap is tagged with (preserving the block id).
    RetagStrap { clock_mhz: u32, new_clock_mhz: u32 },
    /// Set SCLK DPM `level` (MHz).
    PpSclk { level: usize, mhz: u32 },
    /// Set MCLK DPM `level` (MHz).
    PpMclk { level: usize, mhz: u32 },
    /// Set VDDC LUT entry `index` (mV).
    PpVddc { index: usize, mv: u16 },
    /// Set the PowerTune TDP (W).
    PpTdp { watts: u16 },
    /// Write raw bytes at an absolute ROM offset.
    Hex { offset: usize, bytes: Vec<u8> },
}

/// A single byte-range change: where, what it was, what it became.
#[derive(Debug, Clone)]
pub struct Diff {
    pub offset: usize,
    pub old: Vec<u8>,
    pub new: Vec<u8>,
    /// Human-readable description of what the edit means.
    pub label: String,
}

impl Diff {
    pub fn hex_pairs(v: &[u8]) -> String {
        v.iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[derive(Debug, Default)]
pub struct PatchReport {
    pub diffs: Vec<Diff>,
    /// Non-fatal concerns (unusual value, hex inside a parsed table...).
    pub warnings: Vec<String>,
}
