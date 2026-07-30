use anyhow::Result;

use super::reader::Reader;
use super::types::{GpioI2cAssignment, GpioI2cInfo};

const ENTRY_SIZE: usize = 27;

/// Parses the GPIO_I2C_Info data table (`ATOM_GPIO_I2C_INFO`, atombios.h
/// line 3658): one `ATOM_GPIO_I2C_ASSIGMENT` (27 bytes) per I2C/DDC line,
/// giving the GPIO registers used to drive the line (stored as DWORD
/// MMIO register indices - the byte offset is index * 4) and the bit
/// position of each pin in those registers. Matches the interpretation
/// of the radeon kernel driver (radeon_atombios.c, radeon_i2c.c).
pub fn parse_gpio_i2c_info(r: &Reader, off: usize) -> Result<GpioI2cInfo> {
    let struct_size = r.u16(off)?;
    let fmt_rev = r.u8(off + 2)?;
    let cont_rev = r.u8(off + 3)?;

    let avail = (struct_size as usize).saturating_sub(4) / ENTRY_SIZE;
    let mut assignments = Vec::with_capacity(avail);
    for i in 0..avail {
        let p = off + 4 + i * ENTRY_SIZE;
        assignments.push(GpioI2cAssignment {
            clk_mask_reg: r.u16(p)?,
            clk_en_reg: r.u16(p + 2)?,
            clk_y_reg: r.u16(p + 4)?,
            clk_a_reg: r.u16(p + 6)?,
            data_mask_reg: r.u16(p + 8)?,
            data_en_reg: r.u16(p + 10)?,
            data_y_reg: r.u16(p + 12)?,
            data_a_reg: r.u16(p + 14)?,
            i2c_id: r.u8(p + 16)?,
            clk_mask_shift: r.u8(p + 17)?,
            clk_en_shift: r.u8(p + 18)?,
            clk_y_shift: r.u8(p + 19)?,
            clk_a_shift: r.u8(p + 20)?,
            data_mask_shift: r.u8(p + 21)?,
            data_en_shift: r.u8(p + 22)?,
            data_y_shift: r.u8(p + 23)?,
            data_a_shift: r.u8(p + 24)?,
        });
    }

    Ok(GpioI2cInfo {
        struct_size,
        fmt_rev,
        cont_rev,
        assignments,
    })
}
