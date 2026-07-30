pub mod chip;
pub mod display;
pub mod header;
pub mod memory;
pub mod power;

use crate::compare_util;
use crate::render::color::{Palette, fit};
use crate::render::sections::Section;
use crate::rom::types::ParsedRom;

use self::chip::{
    asic_section, gpio_section, i2c_section, power_section, profiling_section, smu_section,
    ss_section, vesa_section,
};
use self::display::{display_section, pcir_section};
use self::header::{firmware_section, header_section};
use self::memory::{caps_section, straps_section, vram_section};
use self::power::{
    fan_section, mclk_section, mm_section, pcie_section, powertune_section, sclk_section,
    voltages_section, vrm_section,
};

/// Column width per ROM: shrinks as the number of ROMs grows,
/// but never becomes illegible or too huge.
fn col_width(n: usize) -> usize {
    (90 / n.max(1)).clamp(10, 26)
}

struct Matrix<'a> {
    pal: &'a Palette,
    diff_only: bool,
    col_w: usize,
    name_w: usize,
    buf: String,
    any_row: bool,
}

impl<'a> Matrix<'a> {
    fn new(pal: &'a Palette, n: usize, diff_only: bool) -> Self {
        Matrix {
            pal,
            diff_only,
            col_w: col_width(n),
            name_w: 24,
            buf: String::new(),
            any_row: false,
        }
    }

    fn header(&mut self, names: &[String]) -> &mut Self {
        let mut line = format!("  {}", fit("field", self.name_w));
        for n in names {
            line.push(' ');
            line.push_str(&fit(n, self.col_w));
        }
        let rule_len = self.name_w + names.len() * (self.col_w + 1) + 2;
        self.buf.push_str(&self.pal.label(&line));
        self.buf.push('\n');
        self.buf.push_str(
            &self
                .pal
                .label(&format!("  {}", "─".repeat(rule_len.saturating_sub(2)))),
        );
        self.buf.push('\n');
        self
    }

    fn row(&mut self, label: &str, values: &[String]) -> &mut Self {
        let all_equal = values.windows(2).all(|w| w[0] == w[1]);
        if self.diff_only && all_equal {
            return self;
        }
        self.any_row = true;
        let marker = if all_equal {
            self.pal.good("=")
        } else {
            self.pal.warn("≠")
        };
        let mut line = format!("  {}", fit(label, self.name_w));
        for v in values {
            line.push(' ');
            line.push_str(&fit(v, self.col_w));
        }
        self.buf.push_str(&format!("{line} {marker}\n"));
        self
    }

    fn note(&mut self, s: &str) -> &mut Self {
        compare_util::note_push(&mut self.buf, &mut self.any_row, s);
        self
    }

    fn finish(self, empty_message: &str) -> String {
        compare_util::finish_buf(self.buf, self.any_row, empty_message)
    }
}

pub fn render_compare_all(
    roms: &[ParsedRom],
    sections: &[Section],
    pal: &Palette,
    diff_only: bool,
) -> String {
    let names: Vec<String> = roms.iter().map(|r| r.file_name.clone()).collect();
    let mut out = Vec::new();
    out.push(pal.accent(&format!(
        "═══ Comparison (matrix): {} ═══",
        names.join(" · ")
    )));
    if diff_only {
        out.push(pal.label("(--diff-only mode: identical lines across all ROMs were omitted)"));
    }
    for &s in sections {
        out.push(String::new());
        out.push(compare_all_section(roms, &names, s, pal, diff_only));
    }
    out.join("\n")
}

fn title(pal: &Palette, s: &str) -> String {
    pal.title(&format!("── {s} "))
}

fn compare_all_section(
    roms: &[ParsedRom],
    names: &[String],
    section: Section,
    pal: &Palette,
    diff_only: bool,
) -> String {
    match section {
        Section::Header => header_section(roms, names, pal, diff_only),
        Section::PcirChain => pcir_section(roms, names, pal, diff_only),
        Section::Display => display_section(roms, names, pal, diff_only),
        Section::Firmware => firmware_section(roms, names, pal, diff_only),
        Section::Sclk => sclk_section(roms, names, pal, diff_only),
        Section::Mclk => mclk_section(roms, names, pal, diff_only),
        Section::Voltages => voltages_section(roms, names, pal, diff_only),
        Section::Vrm => vrm_section(roms, names, pal, diff_only),
        Section::Multimedia => mm_section(roms, names, pal, diff_only),
        Section::Powertune => powertune_section(roms, names, pal, diff_only),
        Section::Fan => fan_section(roms, names, pal, diff_only),
        Section::Pcie => pcie_section(roms, names, pal, diff_only),
        Section::Vram => vram_section(roms, names, pal, diff_only),
        Section::Straps => straps_section(roms, names, pal, diff_only),
        Section::Caps => caps_section(roms, names, pal, diff_only),
        Section::Asic => asic_section(roms, names, pal, diff_only),
        Section::Smu => smu_section(roms, names, pal, diff_only),
        Section::Power => power_section(roms, names, pal, diff_only),
        Section::Gpio => gpio_section(roms, names, pal, diff_only),
        Section::Profiling => profiling_section(roms, names, pal, diff_only),
        Section::Ss => ss_section(roms, names, pal, diff_only),
        Section::Vesa => vesa_section(roms, names, pal, diff_only),
        Section::I2c => i2c_section(roms, names, pal, diff_only),
    }
}
