use super::Table;
use super::title;
use crate::render::color::Palette;
use crate::render::sections::Section;
use crate::rom::types::ParsedRom;

pub(super) fn compare_display(
    a: &ParsedRom,
    b: &ParsedRom,
    pal: &Palette,
    diff_only: bool,
) -> String {
    let mut s = title(pal, Section::Display.label());
    s.push('\n');
    let mut t = Table::new(pal, diff_only);
    t.header(&a.file_name, &b.file_name);
    t.row(
        "Video paths (count)",
        a.display.paths.len(),
        b.display.paths.len(),
    );
    let names_a: Vec<String> = a
        .display
        .paths
        .iter()
        .map(|p| p.device_tag_name.clone())
        .collect();
    let names_b: Vec<String> = b
        .display
        .paths
        .iter()
        .map(|p| p.device_tag_name.clone())
        .collect();
    t.row("Logical devices", names_a.join(", "), names_b.join(", "));
    let n = a.display.paths.len().max(b.display.paths.len());
    for i in 0..n {
        let ca = a
            .display
            .paths
            .get(i)
            .and_then(|p| p.connector.chip_name.clone());
        let cb = b
            .display
            .paths
            .get(i)
            .and_then(|p| p.connector.chip_name.clone());
        t.row(
            &format!("  path {i} connector"),
            ca.unwrap_or_else(|| "-".into()),
            cb.unwrap_or_else(|| "-".into()),
        );
    }
    s.push_str(&t.finish("(nothing differs in this section)"));
    s
}

pub(super) fn compare_pcir_chain(
    a: &ParsedRom,
    b: &ParsedRom,
    pal: &Palette,
    diff_only: bool,
) -> String {
    let mut s = title(pal, Section::PcirChain.label());
    s.push('\n');
    let mut t = Table::new(pal, diff_only);
    t.header(&a.file_name, &b.file_name);
    t.row(
        "Chain images (count)",
        a.pci_images.len(),
        b.pci_images.len(),
    );
    let n = a.pci_images.len().max(b.pci_images.len());
    for i in 0..n {
        let ia = a.pci_images.get(i);
        let ib = b.pci_images.get(i);
        let fmt = |img: &crate::rom::types::PciImage| {
            format!("{} ({} bytes)", img.code_type_name, img.declared_size_bytes)
        };
        t.row(
            &format!("image {i}"),
            ia.map(fmt).unwrap_or_else(|| "-".into()),
            ib.map(fmt).unwrap_or_else(|| "-".into()),
        );
        if let (Some(ia), Some(ib)) = (ia, ib) {
            t.row(
                &format!("  device ID (img. {i})"),
                format!("0x{:04X}", ia.device_id),
                format!("0x{:04X}", ib.device_id),
            );
        }
    }
    s.push_str(&t.finish("(nothing differs in this section)"));
    s
}
