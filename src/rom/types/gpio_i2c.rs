use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct GpioI2cAssignment {
    /// usClkMaskRegisterIndex etc., in DWORD register-index units.
    pub clk_mask_reg: u16,
    pub clk_en_reg: u16,
    pub clk_y_reg: u16,
    pub clk_a_reg: u16,
    pub data_mask_reg: u16,
    pub data_en_reg: u16,
    pub data_y_reg: u16,
    pub data_a_reg: u16,
    /// sucI2cId: bit 7 = HW capable, bits 6:4 = HW engine id, bits 3:0 = line mux.
    pub i2c_id: u8,
    /// Pin bit positions inside the 32-bit GPIO registers.
    pub clk_mask_shift: u8,
    pub clk_en_shift: u8,
    pub clk_y_shift: u8,
    pub clk_a_shift: u8,
    pub data_mask_shift: u8,
    pub data_en_shift: u8,
    pub data_y_shift: u8,
    pub data_a_shift: u8,
}

impl GpioI2cAssignment {
    /// A 32-bit bit-mask from a pin shift.
    pub fn bit_mask(shift: u8) -> u32 {
        1u32 << shift
    }
}

/// `ATOM_GPIO_I2C_INFO` - how the board wires its I2C/DDC lines
/// (GPIO_I2C_Info data table, atombios.h line 3658).
#[derive(Debug, Clone, Serialize)]
pub struct GpioI2cInfo {
    pub struct_size: u16,
    pub fmt_rev: u8,
    pub cont_rev: u8,
    pub assignments: Vec<GpioI2cAssignment>,
}
