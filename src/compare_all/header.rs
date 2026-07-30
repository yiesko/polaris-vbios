use super::Matrix;
use super::title;
use crate::render::color::Palette;
use crate::render::sections::Section;
use crate::rom::types::ParsedRom;

pub(super) fn header_section(
    roms: &[ParsedRom],
    names: &[String],
    pal: &Palette,
    diff_only: bool,
) -> String {
    let mut s = title(pal, Section::Header.label());
    s.push('\n');
    let mut m = Matrix::new(pal, roms.len(), diff_only);
    m.header(names);
    m.row(
        "Size (bytes)",
        &roms
            .iter()
            .map(|r| r.header.file_size.to_string())
            .collect::<Vec<_>>(),
    );
    m.row(
        "PowerPlay format",
        &roms
            .iter()
            .map(|r| {
                format!(
                    "{}.{}",
                    r.powerplay.header_fmt_rev, r.powerplay.header_cont_rev
                )
            })
            .collect::<Vec<_>>(),
    );
    m.row(
        "Subsystem vendor",
        &roms
            .iter()
            .map(|r| {
                r.header
                    .subsystem_vendor_name
                    .clone()
                    .unwrap_or_else(|| format!("0x{:04X}", r.header.subsystem_vendor_id))
            })
            .collect::<Vec<_>>(),
    );
    m.row(
        "Checksum valid",
        &roms
            .iter()
            .map(|r| r.header.checksum_valid.to_string())
            .collect::<Vec<_>>(),
    );
    s.push_str(&m.finish("(nothing differs in this section)"));
    s
}

pub(super) fn firmware_section(
    roms: &[ParsedRom],
    names: &[String],
    pal: &Palette,
    diff_only: bool,
) -> String {
    let mut s = title(pal, Section::Firmware.label());
    s.push('\n');
    let mut m = Matrix::new(pal, roms.len(), diff_only);
    m.header(names);
    m.row(
        "Boot engine (MHz)",
        &roms
            .iter()
            .map(|r| format!("{:.0}", r.firmware.default_engine_clock_mhz))
            .collect::<Vec<_>>(),
    );
    m.row(
        "Boot memory (MHz)",
        &roms
            .iter()
            .map(|r| format!("{:.0}", r.firmware.default_memory_clock_mhz))
            .collect::<Vec<_>>(),
    );
    m.row(
        "Boot VDDC (mV)",
        &roms
            .iter()
            .map(|r| r.firmware.bootup_vddc_mv.to_string())
            .collect::<Vec<_>>(),
    );
    m.row(
        "Boot VDDCI (mV)",
        &roms
            .iter()
            .map(|r| r.firmware.bootup_vddci_mv.to_string())
            .collect::<Vec<_>>(),
    );
    s.push_str(&m.finish("(nothing differs in this section)"));
    s
}
