//! Decoding of GDDR5 memory-strap timing registers (Polaris).
//!
//! The bit layouts follow the community decoders for Polaris/RX straps
//! (OhGodADecode by OhGodACompany, R_Timings by Vento041, and
//! integralfx's BIOSTimingsDecoder) and were cross-checked against four
//! Polaris 10/12 BIOS images (XFX RX 570/580, MSI RX 550): at 2000 MHz
//! they decode to e.g. tRCDW/tRCDWA/tRCDR/tRCDRA = 19/19/27/27, tRRD = 7,
//! tRC = 83, matching the community reference decodes of the same
//! memory-class straps.
//!
//! Timing fields are counts of memory-clock cycles. Physical time is
//! `cycles * 1000 / clock_mhz` nanoseconds; the classic set (tRC, tRFC,
//! tRP, tRRD, tFAW) is rendered with the ns conversion. Registers with
//! no known layout are rendered as raw hex by the callers.

/// One decoded field of a register: name + bit range within the value.
#[derive(Debug, Clone, Copy)]
pub struct TimingField {
    pub name: &'static str,
    pub offset: u32,
    pub width: u32,
    /// Belongs to the classic ns-conversion set (tRC, tRFC, tRP, tRRD, tFAW).
    pub in_ns_set: bool,
}

const fn field(name: &'static str, offset: u32, width: u32) -> TimingField {
    TimingField {
        name,
        offset,
        width,
        in_ns_set: false,
    }
}

const fn field_ns(name: &'static str, offset: u32, width: u32) -> TimingField {
    TimingField {
        name,
        offset,
        width,
        in_ns_set: true,
    }
}

/// A recognized memory-controller register: index + timing layout.
#[derive(Debug, Clone, Copy)]
pub struct TimingRegister {
    pub index: u16,
    pub name: &'static str,
    pub fields: &'static [TimingField],
}

const WR_CTL_D1: TimingRegister = TimingRegister {
    index: 0xA2F,
    name: "WR_CTL_D1",
    fields: &[
        field("DAT_DLY", 0, 4),
        field("DQS_DLY", 4, 4),
        field("OEN_DLY", 16, 4),
        field("ODT_DLY", 26, 4),
    ],
};

const RAS_TIMING: TimingRegister = TimingRegister {
    index: 0xA2C,
    name: "RAS_TIMING",
    fields: &[
        field("tRCDW", 0, 5),
        field("tRCDWA", 5, 5),
        field("tRCDR", 10, 5),
        field("tRCDRA", 15, 5),
        field_ns("tRRD", 20, 4),
        field_ns("tRC", 24, 7),
    ],
};

const CAS_TIMING: TimingRegister = TimingRegister {
    index: 0xA28,
    name: "CAS_TIMING",
    fields: &[
        field("tNOPW", 0, 2),
        field("tNOPR", 2, 2),
        field("tR2W", 4, 5),
        field("tCCDL", 9, 3),
        field("tCCDS", 12, 4),
        field("tW2R", 16, 5),
        field("tCL", 24, 5),
    ],
};

const MISC_TIMING: TimingRegister = TimingRegister {
    index: 0xA29,
    name: "MISC_TIMING",
    fields: &[
        field("tRP_WRA", 0, 7),
        field("tRP_RDA", 7, 7),
        field_ns("tRP", 14, 6),
        field_ns("tRFC", 20, 9),
    ],
};

const MISC_TIMING2: TimingRegister = TimingRegister {
    index: 0xA2A,
    name: "MISC_TIMING2",
    fields: &[
        field("PA2RDATA", 0, 3),
        field("PA2WDATA", 4, 3),
        field_ns("tFAW", 8, 5),
        field("tCRCRL", 13, 3),
        field("tCRCWL", 16, 5),
        field("t32AW", 21, 4),
        field("tWDATATR", 28, 4),
    ],
};

const ARB_DRAM_TIMING: TimingRegister = TimingRegister {
    index: 0xA5F,
    name: "ARB_DRAM_TIMING",
    fields: &[
        field("ACTRD", 0, 8),
        field("ACTWR", 8, 8),
        field("RASMACTRD", 16, 8),
        field("RASMACTWR", 24, 8),
    ],
};

const ARB_DRAM_TIMING2: TimingRegister = TimingRegister {
    index: 0x9DD,
    name: "ARB_DRAM_TIMING2",
    fields: &[
        field("RAS2RAS", 0, 8),
        field("RP", 8, 8),
        field("WRPLUSRP", 16, 8),
        field("BUS_TURN", 24, 8),
    ],
};

/// Recognized memory-controller registers, in the order they appear in
/// the strap data of the Polaris BIOSes we have seen.
pub const KNOWN: &[TimingRegister] = &[
    WR_CTL_D1,
    RAS_TIMING,
    CAS_TIMING,
    MISC_TIMING,
    MISC_TIMING2,
    ARB_DRAM_TIMING,
    ARB_DRAM_TIMING2,
];

/// Layout of the register with the given index, if recognized.
pub fn register(index: u16) -> Option<&'static TimingRegister> {
    KNOWN.iter().find(|r| r.index == index)
}

/// Cycles → nanoseconds at the given memory clock.
pub fn ns(cycles: u32, clock_mhz: f64) -> f64 {
    cycles as f64 * 1000.0 / clock_mhz
}

/// One-line rendering of a strap: decoded `name=cycles` fields in
/// register order (ns in parentheses for the classic set), followed by
/// raw hex for registers with no known layout.
pub fn fmt_strap(values: &[u32], indices: &[u16], clock_mhz: f64) -> String {
    let mut out = String::new();
    let mut raw: Vec<String> = Vec::new();
    for (i, value) in values.iter().enumerate() {
        match indices.get(i) {
            Some(idx) => match register(*idx) {
                Some(reg) => {
                    for f in reg.fields {
                        let cycles = (value >> f.offset) & ((1 << f.width) - 1);
                        if f.in_ns_set {
                            out.push_str(&format!(
                                "{}={} ({} ns) ",
                                f.name,
                                cycles,
                                ns(cycles, clock_mhz).round() as u64
                            ));
                        } else {
                            out.push_str(&format!("{}={} ", f.name, cycles));
                        }
                    }
                }
                None => raw.push(format!("0x{idx:X}=0x{value:08X}")),
            },
            None => raw.push(format!("0x{value:08X}")),
        }
    }
    if !raw.is_empty() {
        out.push_str("· ");
        out.push_str(&raw.join(" "));
    }
    out.trim_end().to_string()
}
