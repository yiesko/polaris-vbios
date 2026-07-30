use anyhow::Result;

use super::reader::Reader;
use super::types::{GpioPinAssignment, GpioPinLut};

/// Pre-defined `ucGPIO_ID` roles (atombios.h line 4375).
fn gpio_id_name(id: u8) -> Option<&'static str> {
    match id {
        56 => Some("PCIE_VDDC control (SLT)"),
        60 => Some("AC/DC switching"),
        61 => Some("VDDC regulator VRHOT"),
        62 => Some("Peak current control (PCC)"),
        63 => Some("EFUSE cut enable"),
        64 => Some("DRAM self-refresh"),
        65 => Some("thermal interrupt output"),
        _ => None,
    }
}

/// Parses `ATOM_GPIO_PIN_LUT` (GPIO_Pin_LUT table, atombios.h line 4386).
/// Each 4-byte `ATOM_GPIO_PIN_ASSIGNMENT` maps a pin index/bit shift to
/// a predefined GPIO role; the entry count comes from the declared
/// structure size.
pub fn parse_gpio_pin_lut(r: &Reader, off: usize) -> Result<GpioPinLut> {
    let struct_size = r.u16(off)?;
    let fmt_rev = r.u8(off + 2)?;
    let cont_rev = r.u8(off + 3)?;

    let pin_base = off + 4;
    let avail = struct_size.saturating_sub(4) / 4;
    let mut pins = Vec::with_capacity(avail as usize);
    for i in 0..avail {
        let p = pin_base + i as usize * 4;
        let id = r.u8(p + 3)?;
        pins.push(GpioPinAssignment {
            index: i as usize,
            gpio_pin_a_index: r.u16(p)?,
            pin_bit_shift: r.u8(p + 2)?,
            gpio_id: id,
            gpio_id_name: gpio_id_name(id).map(str::to_string),
        });
    }

    Ok(GpioPinLut {
        struct_size,
        fmt_rev,
        cont_rev,
        pins,
    })
}
