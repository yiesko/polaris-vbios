use super::Table;
use super::title;
use crate::render::color::Palette;
use crate::render::sections::Section;
use crate::rom::types::ParsedRom;

pub(super) fn compare_header(
    a: &ParsedRom,
    b: &ParsedRom,
    pal: &Palette,
    diff_only: bool,
) -> String {
    let mut s = title(pal, Section::Header.label());
    s.push('\n');
    let mut t = Table::new(pal, diff_only);
    t.header(&a.file_name, &b.file_name);
    t.row("File size (bytes)", a.header.file_size, b.header.file_size);
    t.row(
        "PowerPlay format/revision",
        format!(
            "{}.{}",
            a.powerplay.header_fmt_rev, a.powerplay.header_cont_rev
        ),
        format!(
            "{}.{}",
            b.powerplay.header_fmt_rev, b.powerplay.header_cont_rev
        ),
    );
    t.row(
        "PowerPlay table revision",
        a.powerplay.table_revision,
        b.powerplay.table_revision,
    );
    let vend_a = a
        .header
        .subsystem_vendor_name
        .clone()
        .unwrap_or_else(|| format!("0x{:04X}", a.header.subsystem_vendor_id));
    let vend_b = b
        .header
        .subsystem_vendor_name
        .clone()
        .unwrap_or_else(|| format!("0x{:04X}", b.header.subsystem_vendor_id));
    t.row("Subsystem vendor", vend_a, vend_b);
    t.row(
        "Checksum valid",
        a.header.checksum_valid,
        b.header.checksum_valid,
    );
    t.row(
        "Build date (heuristic)",
        a.header.build_date_candidates.join(" · "),
        b.header.build_date_candidates.join(" · "),
    );
    s.push_str(&t.finish("(nothing differs in this section)"));
    s
}

pub(super) fn compare_firmware(
    a: &ParsedRom,
    b: &ParsedRom,
    pal: &Palette,
    diff_only: bool,
) -> String {
    let (fa, fb) = (&a.firmware, &b.firmware);
    let mut s = title(pal, Section::Firmware.label());
    s.push('\n');
    let mut t = Table::new(pal, diff_only);
    t.header(&a.file_name, &b.file_name);
    t.row_pct(
        "Boot engine clock (MHz)",
        fa.default_engine_clock_mhz,
        fb.default_engine_clock_mhz,
        |v| format!("{v:.0}"),
    );
    t.row_pct(
        "Boot memory clock (MHz)",
        fa.default_memory_clock_mhz,
        fb.default_memory_clock_mhz,
        |v| format!("{v:.0}"),
    );
    t.row_pct(
        "Boot VDDC (mV)",
        fa.bootup_vddc_mv as f64,
        fb.bootup_vddc_mv as f64,
        |v| format!("{v:.0}"),
    );
    t.row_pct(
        "Boot VDDCI (mV)",
        fa.bootup_vddci_mv as f64,
        fb.bootup_vddci_mv as f64,
        |v| format!("{v:.0}"),
    );
    s.push_str(&t.finish("(nothing differs in this section)"));
    s
}
