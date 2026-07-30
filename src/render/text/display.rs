use super::{heading, kv};
use crate::render::color::Palette;
use crate::render::sections::Section;
use crate::rom::types::ParsedRom;

pub(super) fn render_display(rom: &ParsedRom, pal: &Palette) -> String {
    let d = &rom.display;
    let mut s = heading(pal, Section::Display.label());
    s.push('\n');
    s.push_str(&pal.label(
        "Read from ATOM_OBJECT_HEADER → video path table (the modern and \
         authoritative source since the GCN era). Connector/encoder names \
         come verbatim from the official amdgpu driver's ObjectID.h, not \
         from reverse engineering.\n",
    ));

    if !d.paths.is_empty() {
        for (i, p) in d.paths.iter().enumerate() {
            s.push('\n');
            let conn_name = p
                .connector
                .chip_name
                .clone()
                .unwrap_or_else(|| format!("unknown type (0x{:02X})", p.connector.raw & 0xFF));
            s.push_str(&format!(
                "  {} path {} - logical device {} · connector: {}",
                pal.value("▸"),
                i,
                pal.good(&p.device_tag_name),
                conn_name
            ));
            if p.connector.enum_instance > 0 {
                s.push_str(&format!(" (instance {})", p.connector.enum_instance));
            }
            s.push('\n');
            if !p.encoder_chain.is_empty() {
                let chain: Vec<String> = p
                    .encoder_chain
                    .iter()
                    .map(|e| {
                        e.chip_name.clone().unwrap_or_else(|| {
                            format!("{} (0x{:02X})", e.object_type_name, e.raw & 0xFF)
                        })
                    })
                    .collect();
                s.push_str(&format!(
                    "      encoder chain (GPU → connector): {}\n",
                    chain.join(" → ")
                ));
            }
        }
    } else if let Some(bitmap) = d.supported_devices_bitmap {
        s.push('\n');
        s.push_str(&pal.warn(
            "Did not find the modern video path table - showing the older \
             SupportedDevicesInfo bitmap (no physical connector detail):\n",
        ));
        s.push_str(&kv(
            pal,
            "Supported devices (bitmap)",
            format!("0x{bitmap:04X}"),
        ));
        s.push('\n');
        s.push_str(&kv(
            pal,
            "Device names",
            d.supported_devices_names.join(", "),
        ));
    } else {
        s.push_str("\n  (no video output information found in this ROM)");
    }
    s
}

pub(super) fn render_pcir_chain(rom: &ParsedRom, pal: &Palette) -> String {
    let mut s = heading(pal, Section::PcirChain.label());
    if rom.pci_images.is_empty() {
        s.push_str("\n  (no valid PCIR image found)");
        return s;
    }
    s.push('\n');
    s.push_str(&pal.label(
        "Standard PCI Firmware Specification structure - allows more than one \
         firmware image on the same flash chip, one per boot environment. Most \
         modern VBIOS have a legacy (x86) image (with the AtomBIOS that the rest \
         of this program reads) followed by an EFI image (GOP driver for pure UEFI boot).\n",
    ));
    for img in &rom.pci_images {
        s.push('\n');
        s.push_str(&format!(
            "  {} image {} - offset 0x{:X} · {} bytes · {}{}\n",
            pal.value("▸"),
            img.index,
            img.file_offset,
            img.declared_size_bytes,
            img.code_type_name,
            if img.is_last_image {
                pal.label(" (last image)")
            } else {
                String::new()
            }
        ));
        s.push_str(&format!(
            "      PCI vendor:device = {:04X}:{:04X}{} · class code 0x{:06X}{}\n",
            img.vendor_id,
            img.device_id,
            "",
            img.class_code,
            img.class_name
                .as_ref()
                .map(|c| format!(" ({c})"))
                .unwrap_or_default()
        ));
        if img.is_atom_bios {
            s.push_str(&format!("      {} contains a valid AtomBIOS (this is what this program analyzes in detail)\n", pal.good("✓")));
        }
        if let Some(id) = &img.identity_string {
            s.push_str(&format!("      identity string found: \"{id}\"\n"));
        }
    }
    s
}
