use serde::Serialize;

/// `ATOM_GPIO_PIN_ASSIGNMENT` - one GPIO pin and its predefined role.
#[derive(Debug, Clone, Serialize)]
pub struct GpioPinAssignment {
    pub index: usize,
    pub gpio_pin_a_index: u16,
    pub pin_bit_shift: u8,
    pub gpio_id: u8,
    pub gpio_id_name: Option<String>,
}

/// `ATOM_GPIO_PIN_LUT` - GPIO pin roles (VRHOT, AC/DC switch, PCC...).
#[derive(Debug, Clone, Serialize)]
pub struct GpioPinLut {
    pub struct_size: u16,
    pub fmt_rev: u8,
    pub cont_rev: u8,
    pub pins: Vec<GpioPinAssignment>,
}
