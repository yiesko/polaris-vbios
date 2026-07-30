use super::Matrix;
use super::title;
use crate::render::color::Palette;
use crate::render::sections::Section;
use crate::rom::types::ParsedRom;

pub(super) fn display_section(
    roms: &[ParsedRom],
    names: &[String],
    pal: &Palette,
    diff_only: bool,
) -> String {
    let mut s = title(pal, Section::Display.label());
    s.push('\n');
    let mut m = Matrix::new(pal, roms.len(), diff_only);
    m.header(names);
    m.row(
        "Video paths (count)",
        &roms
            .iter()
            .map(|r| r.display.paths.len().to_string())
            .collect::<Vec<_>>(),
    );
    s.push_str(&m.finish("(nothing differs in this section)"));
    s
}

pub(super) fn pcir_section(
    roms: &[ParsedRom],
    names: &[String],
    pal: &Palette,
    diff_only: bool,
) -> String {
    let mut s = title(pal, Section::PcirChain.label());
    s.push('\n');
    let mut m = Matrix::new(pal, roms.len(), diff_only);
    m.header(names);
    m.row(
        "Chain images (count)",
        &roms
            .iter()
            .map(|r| r.pci_images.len().to_string())
            .collect::<Vec<_>>(),
    );
    s.push_str(&m.finish("(nothing differs in this section)"));
    s
}
