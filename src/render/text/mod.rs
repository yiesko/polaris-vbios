pub mod chip;
pub mod display;
pub mod header;
pub mod memory;
pub mod power;

use std::collections::HashMap;

use crate::render::color::Palette;
use crate::render::sections::Section;
use crate::rom::types::ParsedRom;

use self::chip::render_asic;
use self::chip::render_gpio;
use self::chip::render_i2c;
use self::chip::render_power;
use self::chip::render_profiling;
use self::chip::render_smu;
use self::chip::render_ss;
use self::chip::render_vesa;
use self::display::render_display;
use self::display::render_pcir_chain;
use self::header::render_firmware;
use self::header::render_header;
use self::memory::render_caps;
use self::memory::render_straps;
use self::memory::render_vram;
use self::power::render_fan;
use self::power::render_mclk;
use self::power::render_mm;
use self::power::render_pcie;
use self::power::render_powertune;
use self::power::render_sclk;
use self::power::render_voltages;
use self::power::render_vrm;

/// User-supplied register annotations (via --reg-names), used only in
/// the straps section. `None` means "none supplied" - the program never
/// ships with pre-loaded names.
pub type RegNames<'a> = Option<&'a HashMap<u16, String>>;

fn kv(pal: &Palette, k: &str, v: impl std::fmt::Display) -> String {
    format!(
        "  {}  {}",
        pal.label(&format!("{k:<28}")),
        pal.value(&v.to_string())
    )
}

fn heading(pal: &Palette, title: &str) -> String {
    format!("{}\n{}", pal.title(&format!("── {title} ")), "")
}

/// Formats a KiB value as "32 KiB" or, at 1 MiB and above, "8 MiB".
fn fmt_kib(kib: u16) -> String {
    if kib >= 1024 {
        format!("{:.0} MiB", kib as f64 / 1024.0)
    } else {
        format!("{kib} KiB")
    }
}

/// Shared header line for the optional master-data-table sections:
/// shows "table not present" when the ROM has no such table.
fn optional_heading(pal: &Palette, section: Section, present: bool) -> String {
    let mut s = heading(pal, section.label());
    if !present {
        s.push_str(&format!(
            "\n  {} table not present in this ROM.",
            pal.warn("⚠")
        ));
    }
    s
}

/// Sanity warning block (checksum, TDP=0, implausible ranges etc.) -
/// always shown at the top of the dump regardless of which sections
/// were requested via `--sections`, because it is reliability info
/// for what follows, not a data section itself.
pub fn render_warnings(rom: &ParsedRom, pal: &Palette) -> Option<String> {
    if rom.warnings.is_empty() {
        return None;
    }
    let mut s = pal.bad(&format!("⚠ {} validation warning(s):", rom.warnings.len()));
    for w in &rom.warnings {
        s.push_str(&format!("\n  {} {}", pal.bad("•"), w));
    }
    Some(s)
}

pub fn render_sections(
    rom: &ParsedRom,
    sections: &[Section],
    pal: &Palette,
    reg_names: RegNames,
) -> String {
    let mut out = Vec::new();
    out.push(pal.accent(&format!("═══ {} ═══", rom.file_name)));
    if let Some(w) = render_warnings(rom, pal) {
        out.push(String::new());
        out.push(w);
    }
    for &s in sections {
        out.push(String::new());
        out.push(render_section(rom, s, pal, reg_names));
    }
    out.join("\n")
}

pub fn render_section(
    rom: &ParsedRom,
    section: Section,
    pal: &Palette,
    reg_names: RegNames,
) -> String {
    match section {
        Section::Header => render_header(rom, pal),
        Section::PcirChain => render_pcir_chain(rom, pal),
        Section::Display => render_display(rom, pal),
        Section::Firmware => render_firmware(rom, pal),
        Section::Sclk => render_sclk(rom, pal),
        Section::Mclk => render_mclk(rom, pal),
        Section::Voltages => render_voltages(rom, pal),
        Section::Vrm => render_vrm(rom, pal),
        Section::Multimedia => render_mm(rom, pal),
        Section::Powertune => render_powertune(rom, pal),
        Section::Fan => render_fan(rom, pal),
        Section::Pcie => render_pcie(rom, pal),
        Section::Vram => render_vram(rom, pal),
        Section::Straps => render_straps(rom, pal, reg_names),
        Section::Caps => render_caps(rom, pal),
        Section::Asic => render_asic(rom, pal),
        Section::Smu => render_smu(rom, pal),
        Section::Power => render_power(rom, pal),
        Section::Gpio => render_gpio(rom, pal),
        Section::Profiling => render_profiling(rom, pal),
        Section::Ss => render_ss(rom, pal),
        Section::Vesa => render_vesa(rom, pal),
        Section::I2c => render_i2c(rom, pal),
    }
}
