//! Power sanity envelopes per Polaris die family.
//!
//! The single source of truth for "what TDP makes sense for this
//! silicon", shared by `validate` (read path - warnings) and `patch`
//! (write path - refuses absurd values unless `--force`). The ranges
//! are measured from stock VBIOS (TechPowerUp database, hundreds of
//! factory ROMs) - not a theoretical spec sheet, so factory SKUs never
//! trip the unusual warning:
//!
//! - Baffin 42-75 W: RX 460 = 48 W, RX 560/560D = 42-60 W;
//! - Lexa 35-65 W: RX 550 = 35 W, RX 560 (P12) = 42-60 W;
//! - Ellesmere 10 85 W (RX 470) .. 130 W (RX 480/nitro OC);
//! - Ellesmere 20 60-185 W: RX 570 = 120 W, RX 580 = 145-180 W,
//!   notebooks = 68-85 W;
//! - Polaris 30 185-220 W: RX 590 = 185 W, XFX 196 W, MSI 220 W.
//!
//! - stock range: the actual factory TDPs of that die (including
//!   mobile and OEM variants);
//! - reported ceiling: the highest real-world, multi-report power
//!   limit observed in the field (never the theoretical connector
//!   ceiling);
//! - a value above `reported_oc_max + 25%` is physically implausible:
//!   the VRM/phases/cooler of a Polaris board were never sized for it
//!   (e.g. 300 W in a 120 W RX 570 single 6-pin ROM).

use super::types::ParsedRom;

/// Polaris die families, from the physical die (several marketing
/// names map to one die: Ellesmere = Polaris 10, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Die {
    /// Polaris 11 - RX 460/560 (Baffin).
    Baffin,
    /// Polaris 12 - RX 550 (Lexa).
    Lexa,
    /// Polaris 10 - RX 470/480 (Ellesmere).
    Ellesmere10,
    /// Polaris 20 - RX 570/580 (Ellesmere refresh).
    Ellesmere20,
    /// Polaris 30 - RX 590 (12nm refresh).
    Polaris30,
    /// Ellesmere die (RX 470/480/570/580) whose boot string does not
    /// say whether it is Polaris 10 or Polaris 20 - judged against the
    /// union of both generations.
    EllesmereGeneric,
    /// Device ID not recognized as Polaris.
    Unknown,
}

impl Die {
    pub fn label(self) -> &'static str {
        match self {
            Die::Baffin => "RX 460/560 (Polaris 11, Baffin)",
            Die::Lexa => "RX 550 (Polaris 12, Lexa)",
            Die::Ellesmere10 => "RX 470/480 (Polaris 10, Ellesmere)",
            Die::Ellesmere20 => "RX 570/580 (Polaris 20)",
            Die::Polaris30 => "RX 590 (Polaris 30)",
            Die::EllesmereGeneric => "RX 470-580 (Ellesmere, P10/P20)",
            Die::Unknown => "unknown die",
        }
    }
}

/// TDP envelope of one die family, in watts.
#[derive(Debug, Clone, Copy)]
pub struct PowerEnvelope {
    /// Lowest factory SKU of that die.
    pub stock_min_w: u32,
    /// Highest factory SKU / factory-OC ("OC+") of that die.
    pub stock_max_w: u32,
    /// Highest real-world reported power limit (not theoretical).
    pub reported_oc_max_w: u32,
}

/// TDP envelope per die; `None` for an unrecognized die (no sanity
/// data to judge against).
pub fn envelope_for(die: Die) -> Option<PowerEnvelope> {
    Some(match die {
        Die::Baffin => PowerEnvelope {
            stock_min_w: 42,
            stock_max_w: 75,
            reported_oc_max_w: 90,
        },
        Die::Lexa => PowerEnvelope {
            stock_min_w: 35,
            stock_max_w: 65,
            reported_oc_max_w: 75,
        },
        Die::Ellesmere10 => PowerEnvelope {
            stock_min_w: 85,
            stock_max_w: 130,
            reported_oc_max_w: 150,
        },
        Die::Ellesmere20 => PowerEnvelope {
            stock_min_w: 60,
            stock_max_w: 185,
            reported_oc_max_w: 210,
        },
        Die::Polaris30 => PowerEnvelope {
            stock_min_w: 185,
            stock_max_w: 220,
            reported_oc_max_w: 235,
        },
        Die::EllesmereGeneric => PowerEnvelope {
            stock_min_w: 85,
            stock_max_w: 185,
            reported_oc_max_w: 210,
        },
        Die::Unknown => return None,
    })
}

/// Above this, a TDP is physically implausible: 25% headroom over the
/// highest power anyone reported driving through that die.
pub fn absurd_ceiling(env: &PowerEnvelope) -> u32 {
    env.reported_oc_max_w + env.reported_oc_max_w / 4
}

/// How a TDP relates to a die's envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TdpKind {
    /// Inside the factory range: nothing to say.
    Normal,
    /// Below every factory SKU, but not half of the floor: suspicious.
    UnusualLow,
    /// Above every factory SKU but within what real OC reports reach.
    UnusualHigh,
    /// Below half the factory floor: cannot be a real power limit.
    TooLow,
    /// Above the +25% OC ceiling: physically implausible.
    Absurd,
}

pub fn classify(watts: u32, env: &PowerEnvelope) -> TdpKind {
    if watts < env.stock_min_w / 2 {
        TdpKind::TooLow
    } else if watts < env.stock_min_w {
        TdpKind::UnusualLow
    } else if watts > absurd_ceiling(env) {
        TdpKind::Absurd
    } else if watts > env.stock_max_w {
        TdpKind::UnusualHigh
    } else {
        TdpKind::Normal
    }
}

/// Rejected TDP: below the floor or above the OC ceiling.
#[derive(Debug, Clone, Copy)]
pub struct TdpError {
    pub watts: u32,
    pub die: Die,
    pub kind: TdpKind,
}

impl TdpError {
    pub fn message(&self) -> String {
        let Some(env) = envelope_for(self.die) else {
            return format!("TDP {} W for an unrecognized die", self.watts);
        };
        match self.kind {
            TdpKind::TooLow => format!(
                "TDP {} W is below half of the {} W floor of a {} - no Polaris card of \
                 that die runs below {} W (nominal range {}-{} W)",
                self.watts,
                env.stock_min_w,
                self.die.label(),
                env.stock_min_w,
                env.stock_min_w,
                env.stock_max_w
            ),
            TdpKind::Absurd => format!(
                "TDP {} W is physically implausible for a {}: nominal range is {}-{} W and \
                 real-world OC tops out near {} W (+25% ceiling {}) - the ROM may be \
                 corrupted, from another GPU, or a mining edit",
                self.watts,
                self.die.label(),
                env.stock_min_w,
                env.stock_max_w,
                env.reported_oc_max_w,
                absurd_ceiling(&env)
            ),
            // Normal/UnusualLow/UnusualHigh are not error states; the
            // constructor only creates TdpError for TooLow/Absurd.
            _ => String::new(),
        }
    }
}

/// A TDP that passed the die envelope: either inside the factory
/// range or within the reported-OC headroom (the latter flagged by
/// [`SafeTdp::is_unusual`]). Rejected (TooLow/Absurd) values never
/// become one - the caller decides between refusing and `--force`.
#[derive(Debug, Clone, Copy)]
pub struct SafeTdp {
    watts: u32,
    die: Die,
}

impl SafeTdp {
    pub fn try_new(watts: u32, die: Die) -> Result<SafeTdp, TdpError> {
        match envelope_for(die) {
            None => Ok(SafeTdp { watts, die }),
            Some(env) => {
                let kind = classify(watts, &env);
                match kind {
                    TdpKind::TooLow | TdpKind::Absurd => Err(TdpError { watts, die, kind }),
                    _ => Ok(SafeTdp { watts, die }),
                }
            }
        }
    }

    /// Outside the factory range but within what real OC reaches:
    /// allowed, but the caller should warn loudly.
    pub fn is_unusual(self) -> bool {
        let Some(env) = envelope_for(self.die) else {
            return false;
        };
        matches!(
            classify(self.watts, &env),
            TdpKind::UnusualLow | TdpKind::UnusualHigh
        )
    }

    pub fn unusual_message(self) -> String {
        let env = envelope_for(self.die).expect("unusual requires a known die");
        if self.watts < env.stock_min_w {
            format!(
                "TDP {} W is below the {} W factory floor of a {} (nominal {}-{} W)",
                self.watts,
                env.stock_min_w,
                self.die.label(),
                env.stock_min_w,
                env.stock_max_w
            )
        } else {
            format!(
                "TDP {} W is above the {} W factory range of a {} (real-world OC reports reach \
                 ~{} W)",
                self.watts,
                env.stock_max_w,
                self.die.label(),
                env.reported_oc_max_w
            )
        }
    }
}

/// Detects the die family of a parsed ROM. The device ID separates
/// Baffin (0x67EF), Lexa (0x67FF/0x699F) and the Ellesmere family
/// (0x67DF). The three Ellesmere variants all share device 0x67DF, so
/// the distinction needs extra signals:
///
/// 1. the BIOS bootup message names the die (e.g. "D00034 Polaris20 XL
///    A1" or "C94441 POLARIS 30 XT A1" - whitespace/hyphens are
///    ignored so "POLARIS 30" is not missed);
/// 2. otherwise the MC microcode version separates Polaris 30 (12 nm)
///    from the 14 nm dies;
/// 3. otherwise the boot string does not say (Asus "67DFHB...", MSI
///    "113-MSI...", Gigabyte "GV-...", Sapphire "E347/E353..."): judged
///    against the union of Polaris 10 + Polaris 20 (RX 470..580).
pub fn detect_die(rom: &ParsedRom) -> Die {
    let device = rom.pci_images.first().map(|img| img.device_id);
    match device {
        Some(0x67EF) => Die::Baffin,
        Some(0x67FF) | Some(0x699F) => Die::Lexa,
        Some(0x67DF) => {
            let msg = rom
                .header
                .bios_bootup_message
                .as_deref()
                .unwrap_or_default();
            if contains_ignore_ws(msg, "polaris30") {
                Die::Polaris30
            } else if contains_ignore_ws(msg, "polaris20") {
                Die::Ellesmere20
            } else if contains_ignore_ws(msg, "polaris10") || contains_ignore_ws(msg, "ellesmere") {
                Die::Ellesmere10
            } else if rom.vram.mcu_code_version.is_some_and(|v| v >= 11_853_696) {
                Die::Polaris30
            } else {
                Die::EllesmereGeneric
            }
        }
        _ => Die::Unknown,
    }
}

/// Returns `true` if `haystack` contains `needle` when all whitespace
/// and hyphens are stripped from both sides (case-insensitive).
fn contains_ignore_ws(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let needle_lower: Vec<u8> = needle.bytes().map(|b| b.to_ascii_lowercase()).collect();
    let mut matched = 0usize;
    for &b in haystack.as_bytes() {
        match b {
            b' ' | b'-' | b'\t' | b'\n' | b'\r' => continue,
            b if b.to_ascii_lowercase() == needle_lower[matched] => {
                matched += 1;
                if matched == needle_lower.len() {
                    return true;
                }
            }
            _ => {
                // Check if current byte matches the start of needle.
                matched = if b.to_ascii_lowercase() == needle_lower[0] {
                    1
                } else {
                    0
                };
            }
        }
    }
    false
}
