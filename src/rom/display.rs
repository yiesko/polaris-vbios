use anyhow::Result;

use super::header::master_table_offset;
use super::reader::Reader;
use super::types::{DisplayInfo, DisplayPath, EncoderRef};

const OBJECT_ID_MASK: u16 = 0x00FF;
const ENUM_ID_MASK: u16 = 0x0700;
const OBJECT_TYPE_MASK: u16 = 0x7000;
const ENUM_ID_SHIFT: u16 = 8;
const OBJECT_TYPE_SHIFT: u16 = 12;

fn object_type_name(t: u8) -> &'static str {
    match t {
        0x1 => "GPU",
        0x2 => "Encoder",
        0x3 => "Connector",
        0x4 => "Router",
        0x6 => "Display Path",
        0x7 => "Generic",
        _ => "unknown",
    }
}

/// Names come verbatim from the `amdgpu` driver's `ObjectID.h` (same
/// official source used for voltage regulator names) - not community
/// reverse engineering.
fn connector_name(id: u8) -> Option<&'static str> {
    Some(match id {
        0x01 => "DVI-I (single link)",
        0x02 => "DVI-I (dual link)",
        0x03 => "DVI-D (single link)",
        0x04 => "DVI-D (dual link)",
        0x05 => "VGA",
        0x06 => "Composite",
        0x07 => "S-Video",
        0x08 => "YPbPr (component)",
        0x09 => "D-Connector",
        0x0A => "9-pin DIN",
        0x0B => "SCART",
        0x0C => "HDMI type A",
        0x0D => "HDMI type B",
        0x0E => "LVDS",
        0x0F => "7-pin DIN",
        0x10 => "PCIe (connector)",
        0x11 => "CrossFire",
        0x12 => "DVI (hardcoded)",
        0x13 => "DisplayPort",
        0x14 => "eDP",
        0x15 => "MXM",
        0x16 => "LVDS/eDP",
        0x17 => "USB-C",
        _ => return None,
    })
}

fn encoder_name(id: u8) -> Option<&'static str> {
    Some(match id {
        0x01 => "Internal LVDS",
        0x02 => "Internal TMDS1",
        0x03 => "Internal TMDS2",
        0x04 => "Internal DAC1",
        0x05 => "Internal DAC2 (TV/CV)",
        0x06 => "Internal SDVOA",
        0x07 => "Internal SDVOB",
        0x08 => "SI170B (external)",
        0x09 => "CH7303 (external)",
        0x0A => "CH7301 (external)",
        0x0B => "Internal DVO1",
        0x0C => "External SDVOA",
        0x0D => "External SDVOB",
        0x0E => "TITFP513 (external)",
        0x0F => "Internal LVTM1",
        0x10 => "VT1623 (external)",
        0x11 => "HDMI SI1930 (external)",
        0x12 => "Internal HDMI",
        0x13 => "KLDSCP TMDS1 internal",
        0x14 => "KLDSCP DVO1 internal",
        0x15 => "KLDSCP DAC1 internal",
        0x16 => "KLDSCP DAC2 internal (TV/CV/CRT)",
        0x17 => "SI178 (external, dual-link TMDS)",
        0x18 => "MVPU FPGA",
        0x19 => "Internal DDI",
        0x1A => "VT1625 (external)",
        0x1B => "HDMI SI1932 (external)",
        0x1C => "DisplayPort AN9801 (external)",
        0x1D => "DisplayPort DP501 (external)",
        0x1E => "Internal UniPHY",
        0x1F => "KLDSCP LVTMA internal",
        0x20 => "Internal UniPHY1",
        0x21 => "Internal UniPHY2",
        0x22 => "Almond/Nutmeg (external)",
        0x23 => "Travis (external)",
        0x24 => "Internal VCE",
        0x25 => "Internal UniPHY3",
        0x26 => "HDMI ANX9805 (external)",
        0x27 => "Internal AMCLK",
        0xFF => "Generic external DVO",
        _ => return None,
    })
}

fn device_tag_name(tag: u16) -> String {
    let name = match tag {
        0x0001 => "CRT1",
        0x0002 => "LCD1",
        0x0004 => "TV1",
        0x0008 => "DFP1",
        0x0010 => "CRT2",
        0x0020 => "LCD2",
        0x0040 => "DFP6",
        0x0080 => "DFP2",
        0x0100 => "CV",
        0x0200 => "DFP3",
        0x0400 => "DFP4",
        0x0800 => "DFP5",
        _ => return format!("unknown (0x{tag:04X})"),
    };
    name.to_string()
}

fn decode_object_id(raw: u16) -> EncoderRef {
    let object_type_raw = ((raw & OBJECT_TYPE_MASK) >> OBJECT_TYPE_SHIFT) as u8;
    let object_id = (raw & OBJECT_ID_MASK) as u8;
    let enum_instance = (raw & ENUM_ID_MASK) >> ENUM_ID_SHIFT;
    let chip_name = match object_type_raw {
        0x2 => encoder_name(object_id).map(str::to_string),
        0x3 => connector_name(object_id).map(str::to_string),
        _ => None,
    };
    EncoderRef {
        raw,
        object_type_raw,
        object_type_name: object_type_name(object_type_raw).to_string(),
        enum_instance,
        chip_name,
    }
}

fn parse_display_path_table(r: &Reader, table_off: usize) -> Result<Vec<DisplayPath>> {
    let n = r.u8(table_off)? as usize;
    let mut paths = Vec::with_capacity(n);
    let mut p = table_off + 4;
    for _ in 0..n.min(16) {
        if p + 8 > r.len() {
            break;
        }
        let device_tag_raw = r.u16(p)?;
        let size = r.u16(p + 2)? as usize;
        let conn_obj_id = r.u16(p + 4)?;
        let n_graphic_objs = size.saturating_sub(8) / 2;
        let mut encoder_chain = Vec::with_capacity(n_graphic_objs);
        let mut q = p + 8;
        for _ in 0..n_graphic_objs.min(8) {
            if q + 2 > r.len() {
                break;
            }
            encoder_chain.push(decode_object_id(r.u16(q)?));
            q += 2;
        }
        paths.push(DisplayPath {
            device_tag_raw,
            device_tag_name: device_tag_name(device_tag_raw),
            connector: decode_object_id(conn_obj_id),
            encoder_chain,
        });
        if size < 8 {
            break;
        }
        p += size;
    }
    Ok(paths)
}

fn parse_supported_devices_bitmap(r: &Reader, off: usize) -> Result<(u16, Vec<String>)> {
    let bitmap = r.u16(off + 4)?;
    let all_tags: [u16; 12] = [
        0x0001, 0x0002, 0x0004, 0x0008, 0x0010, 0x0020, 0x0040, 0x0080, 0x0100, 0x0200, 0x0400,
        0x0800,
    ];
    let names = all_tags
        .iter()
        .filter(|&&t| bitmap & t != 0)
        .map(|&t| device_tag_name(t))
        .collect();
    Ok((bitmap, names))
}

/// Reads video output (connectors and encoder chain to GPU) from
/// `ATOM_OBJECT_HEADER` → Display Path Table - the modern and
/// authoritative source for this since the GCN era. As a fallback (only
/// if the object header table does not exist or comes empty), uses the
/// simple `SupportedDevicesInfo` bitmap (older, but still present in
/// some ROMs for compatibility).
pub fn parse_display_info(r: &Reader, mdt_offset: usize) -> Result<DisplayInfo> {
    let obj_header_off = master_table_offset(r, mdt_offset, "Object_Header").unwrap_or(0);

    let mut paths = Vec::new();
    let mut recognized_format = false;

    if obj_header_off != 0 {
        let fmt_rev = r.u8(obj_header_off + 2)?;
        let display_path_rel = r.u16(obj_header_off + 14)? as usize;
        recognized_format = fmt_rev >= 1;
        if display_path_rel != 0 {
            paths = parse_display_path_table(r, obj_header_off + display_path_rel)?;
        }
    }

    let (supported_devices_bitmap, supported_devices_names) = if paths.is_empty() {
        let sdi_off = master_table_offset(r, mdt_offset, "SupportedDevicesInfo").unwrap_or(0);
        if sdi_off != 0 {
            let (bitmap, names) = parse_supported_devices_bitmap(r, sdi_off)?;
            (Some(bitmap), names)
        } else {
            (None, Vec::new())
        }
    } else {
        (None, Vec::new())
    };

    Ok(DisplayInfo {
        recognized_format,
        paths,
        supported_devices_bitmap,
        supported_devices_names,
    })
}
