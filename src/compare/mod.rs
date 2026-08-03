pub mod chip;
pub mod display;
pub mod header;
pub mod memory;
pub mod power;

use crate::rom::types::ParsedRom;

use crate::compare_util;
use crate::render::color::{Palette, fit};
use crate::render::sections::Section;
use crate::render::text::RegNames;

use self::chip::{
    compare_asic, compare_gpio, compare_i2c, compare_power, compare_profiling, compare_smu,
    compare_ss, compare_vesa,
};
use self::display::{compare_display, compare_pcir_chain};
use self::header::{compare_firmware, compare_header};
use self::memory::{compare_caps, compare_straps, compare_vram};
use self::power::{
    compare_fan, compare_mclk, compare_mm, compare_pcie, compare_powertune, compare_sclk,
    compare_voltages, compare_vrm,
};

const NAME_W: usize = 28;
const VAL_W: usize = 26;

fn header_row(pal: &Palette, name_a: &str, name_b: &str) -> String {
    // Build the line fully aligned in plain text first, then apply
    // color once to the final result - so ANSI codes never enter the
    // column width calculation.
    let line = format!(
        "  {} {} {}",
        fit("field", NAME_W),
        fit(name_a, VAL_W),
        fit(name_b, VAL_W)
    );
    let rule = format!("  {}", "─".repeat(NAME_W + VAL_W * 2 + 2));
    format!("{}\n{}", pal.label(&line), pal.label(&rule))
}

/// Accumulates comparison table lines, with support for `--diff-only`
/// (omit rows where both values are equal) and percent delta for
/// numeric fields - centralized here to avoid repeating the logic in
/// each section.
struct Table<'a> {
    pal: &'a Palette,
    diff_only: bool,
    buf: String,
    any_row: bool,
}

impl<'a> Table<'a> {
    fn new(pal: &'a Palette, diff_only: bool) -> Self {
        Table {
            pal,
            diff_only,
            buf: String::new(),
            any_row: false,
        }
    }

    fn header(&mut self, name_a: &str, name_b: &str) -> &mut Self {
        self.buf.push_str(&header_row(self.pal, name_a, name_b));
        self.buf.push('\n');
        self
    }

    /// Generic comparison row (text or number without % delta).
    fn row(
        &mut self,
        label: &str,
        a: impl std::fmt::Display,
        b: impl std::fmt::Display,
    ) -> &mut Self {
        let a_s = a.to_string();
        let b_s = b.to_string();
        if self.diff_only && a_s == b_s {
            return self;
        }
        self.any_row = true;
        let differs = a_s != b_s;
        let marker = if differs {
            self.pal.warn("≠")
        } else {
            self.pal.good("=")
        };
        self.buf.push_str(&format!(
            "  {} {} {} {}\n",
            fit(label, NAME_W),
            fit(&a_s, VAL_W),
            fit(&b_s, VAL_W),
            marker
        ));
        self
    }

    /// Numeric row with percent delta embedded in B's value - for
    /// TDP, TDC, clocks, voltages, temperatures, etc.
    fn row_pct(&mut self, label: &str, a: f64, b: f64, fmt: impl Fn(f64) -> String) -> &mut Self {
        let equal = (a - b).abs() < 1e-9;
        if self.diff_only && equal {
            return self;
        }
        self.any_row = true;
        let a_s = fmt(a);
        let b_s = if equal {
            fmt(b)
        } else {
            format!("{} ({})", fmt(b), compare_util::pct_delta(a, b))
        };
        let marker = if equal {
            self.pal.good("=")
        } else {
            self.pal.warn("≠")
        };
        self.buf.push_str(&format!(
            "  {} {} {} {}\n",
            fit(label, NAME_W),
            fit(&a_s, VAL_W),
            fit(&b_s, VAL_W),
            marker
        ));
        self
    }

    /// Free text (note, warning) - always shown, even in diff-only,
    /// because it is not a comparable data row.
    fn note(&mut self, s: &str) -> &mut Self {
        compare_util::note_push(&mut self.buf, &mut self.any_row, s);
        self
    }

    fn finish(self, empty_message: &str) -> String {
        compare_util::finish_buf(self.buf, self.any_row, empty_message)
    }
}

pub fn render_compare(
    a: &ParsedRom,
    b: &ParsedRom,
    sections: &[Section],
    pal: &Palette,
    diff_only: bool,
    reg_names: RegNames,
) -> String {
    let mut out = Vec::new();
    out.push(pal.accent(&format!(
        "═══ Comparison: {} vs {} ═══",
        a.file_name, b.file_name
    )));
    if diff_only {
        out.push(pal.label("(--diff-only mode: identical lines in both ROMs were omitted)"));
    }
    for &s in sections {
        out.push(String::new());
        out.push(compare_section(a, b, s, pal, diff_only, reg_names));
    }
    out.join("\n")
}

fn compare_section(
    a: &ParsedRom,
    b: &ParsedRom,
    section: Section,
    pal: &Palette,
    diff_only: bool,
    reg_names: RegNames,
) -> String {
    match section {
        Section::Header => compare_header(a, b, pal, diff_only),
        Section::PcirChain => compare_pcir_chain(a, b, pal, diff_only),
        Section::Display => compare_display(a, b, pal, diff_only),
        Section::Firmware => compare_firmware(a, b, pal, diff_only),
        Section::Sclk => compare_sclk(a, b, pal, diff_only),
        Section::Mclk => compare_mclk(a, b, pal, diff_only),
        Section::Voltages => compare_voltages(a, b, pal, diff_only),
        Section::Vrm => compare_vrm(a, b, pal, diff_only),
        Section::Multimedia => compare_mm(a, b, pal, diff_only),
        Section::Powertune => compare_powertune(a, b, pal, diff_only),
        Section::Fan => compare_fan(a, b, pal, diff_only),
        Section::Pcie => compare_pcie(a, b, pal, diff_only),
        Section::Vram => compare_vram(a, b, pal, diff_only),
        Section::Straps => compare_straps(a, b, pal, diff_only, reg_names),
        Section::Caps => compare_caps(a, b, pal, diff_only),
        Section::Asic => compare_asic(a, b, pal, diff_only),
        Section::Smu => compare_smu(a, b, pal, diff_only),
        Section::Power => compare_power(a, b, pal, diff_only),
        Section::Gpio => compare_gpio(a, b, pal, diff_only),
        Section::Profiling => compare_profiling(a, b, pal, diff_only),
        Section::Ss => compare_ss(a, b, pal, diff_only),
        Section::Vesa => compare_vesa(a, b, pal, diff_only),
        Section::I2c => compare_i2c(a, b, pal, diff_only),
    }
}

fn title(pal: &Palette, s: &str) -> String {
    pal.title(&format!("── {s} "))
}

/// Whether a rendered comparison reports at least one differing row.
/// Data rows always end with their marker (`≠` for a difference, `=`
/// for an equal row), while notes/legends never do - so a line ending
/// in `≠` is the report's own verdict. ANSI codes are stripped first
/// (colors wrap the marker). Used by `compare` to script the exit code.
pub fn differs(content: &str) -> bool {
    let plain = strip_ansi_escapes::strip(content);
    String::from_utf8_lossy(&plain)
        .lines()
        .any(|l| l.trim_end().ends_with('≠'))
}
