use anyhow::Result;

use super::reader::Reader;
use super::types::PciImage;

fn code_type_name(t: u8) -> &'static str {
    match t {
        0x00 => "x86 legacy (PC-AT compatible)",
        0x01 => "Open Firmware",
        0x02 => "PA-RISC",
        0x03 => "EFI Image (typically UEFI GOP)",
        _ => "unknown/reserved",
    }
}

/// Decodes only the base class (most significant byte of the 3-byte
/// class code) for the most common cases in GPU option ROMs - not a
/// complete PCI-SIG table, just enough to confirm "yup, this is a
/// display controller".
fn base_class_name(class_code: u32) -> Option<&'static str> {
    let base = (class_code >> 16) & 0xFF;
    Some(match base {
        0x00 => "Pre-2.0 device",
        0x01 => "Mass storage controller",
        0x02 => "Network controller",
        0x03 => "Display controller",
        0x04 => "Multimedia controller",
        0x06 => "Bridge",
        0x0C => "Serial controller",
        _ => return None,
    })
}

/// Scans just after the PCIR header of an EFI image for a short ASCII
/// string that typically identifies the driver (e.g. "GOP AMD REV:
/// 1.70"). This is a best-effort scan - not every EFI image will have
/// this, and when it does not, no string appears (nothing is made up).
fn scan_efi_identity_string(r: &Reader, search_start: usize, search_len: usize) -> Option<String> {
    let end = (search_start + search_len).min(r.len());
    let mut i = search_start;
    let mut best: Option<String> = None;
    while i + 4 < end {
        let b = r.u8(i).ok()?;
        if b.is_ascii_graphic() {
            let mut j = i;
            let mut s = String::new();
            while j < end {
                let c = r.u8(j).unwrap_or(0);
                if c.is_ascii_graphic() || c == b' ' {
                    s.push(c as char);
                    j += 1;
                } else {
                    break;
                }
            }
            if s.len() >= 6 && (best.as_ref().map(|b| s.len() > b.len()).unwrap_or(true)) {
                best = Some(s.clone());
            }
            i = j.max(i + 1);
        } else {
            i += 1;
        }
    }
    best
}

/// Walks the PCI Option ROM image chain (PCIR structure, defined by
/// the PCI Firmware Specification - not AMD-specific, it is the
/// standard mechanism that allows multiple firmware images to coexist
/// on the same flash chip, one per boot environment type). Most modern
/// VBIOS have exactly 2: the legacy x86 image (which is the
/// `ATOM_ROM_HEADER` that the rest of this program reads in detail)
/// followed by an EFI image (the GOP driver used in pure UEFI boot,
/// without CSM/legacy).
pub fn walk_pci_images(r: &Reader) -> Result<Vec<PciImage>> {
    let mut images = Vec::new();
    let mut offset = 0usize;
    let mut index = 0usize;

    loop {
        if index >= 8 || offset + 0x1A > r.len() {
            break;
        }
        let legacy_sig_valid = r.u8(offset)? == 0x55 && r.u8(offset + 1)? == 0xAA;
        if !legacy_sig_valid {
            if index == 0 {
                // the first image should always have the signature -
                // this is already checked elsewhere before reaching here,
                // but it does not hurt to not break if called in isolation.
            }
            break;
        }

        let pcir_rel = r.u16(offset + 0x18)? as usize;
        if pcir_rel == 0 || offset + pcir_rel + 0x18 > r.len() {
            break;
        }
        let pcir_off = offset + pcir_rel;
        let sig = r.bytes(pcir_off, 4)?;
        if sig != b"PCIR" && sig != b"NPDS" {
            break;
        }

        let vendor_id = r.u16(pcir_off + 4)?;
        let device_id = r.u16(pcir_off + 6)?;
        let pcir_struct_length = r.u16(pcir_off + 0xA)?;
        let pcir_struct_revision = r.u8(pcir_off + 0xC)?;
        let class_code = (r.u8(pcir_off + 0xD)? as u32)
            | ((r.u8(pcir_off + 0xE)? as u32) << 8)
            | ((r.u8(pcir_off + 0xF)? as u32) << 16);
        let image_len_units = r.u16(pcir_off + 0x10)?;
        let revision_level = r.u16(pcir_off + 0x12)?;
        let code_type = r.u8(pcir_off + 0x14)?;
        let indicator = r.u8(pcir_off + 0x15)?;
        let is_last_image = indicator & 0x80 != 0;
        let declared_size_bytes = image_len_units as usize * 512;

        let is_atom_bios = code_type == 0x00 && {
            let atom_ptr_field = offset + 0x48;
            if atom_ptr_field + 2 <= r.len() {
                r.u16(atom_ptr_field)
                    .ok()
                    .and_then(|p| {
                        let abs = offset + p as usize;
                        r.bytes(abs + 4, 4).ok().map(|s| s == b"ATOM")
                    })
                    .unwrap_or(false)
            } else {
                false
            }
        };

        let identity_string = if code_type == 0x03 {
            scan_efi_identity_string(r, pcir_off + 0x16, 256)
        } else {
            None
        };

        images.push(PciImage {
            index,
            file_offset: offset,
            pcir_offset: pcir_off,
            vendor_id,
            device_id,
            class_code,
            class_name: base_class_name(class_code).map(str::to_string),
            declared_size_bytes,
            revision_level,
            code_type,
            code_type_name: code_type_name(code_type).to_string(),
            is_last_image,
            is_atom_bios,
            identity_string,
            pcir_struct_length,
            pcir_struct_revision,
        });

        if is_last_image || declared_size_bytes == 0 {
            break;
        }
        offset += declared_size_bytes;
        index += 1;
    }

    Ok(images)
}
