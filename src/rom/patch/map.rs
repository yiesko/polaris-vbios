use anyhow::{Result, bail};

use crate::rom::header;
use crate::rom::reader::Reader;

/// Resolved table offsets of a ROM image.
pub(super) struct RomMap {
    pub atom_ptr: usize,
    pub mdt: usize,
    pub mct: usize,
    pub pp_off: Option<usize>,
    pub vram_off: Option<usize>,
    pub declared_bytes: usize,
}

pub(super) fn map_rom(data: &[u8]) -> Result<RomMap> {
    let r = Reader::new(data);
    if r.len() < 4 || r.u8(0)? != 0x55 || r.u8(1)? != 0xAA {
        bail!("not a legacy BIOS image (missing 0x55 0xAA)");
    }
    let atom_ptr = r.u16(0x48)? as usize;
    let sig = r.bytes(atom_ptr + 4, 4)?;
    if sig != b"ATOM" {
        bail!("ATOM header not found at 0x{atom_ptr:X}");
    }
    let mdt = r.u16(atom_ptr + 0x20)? as usize;
    let mct = r.u16(atom_ptr + 0x1e)? as usize;
    let pp_off = header::master_table_offset(&r, mdt, "PowerPlayInfo").ok();
    let vram_off = header::master_table_offset(&r, mdt, "VRAM_Info").ok();
    let declared_bytes = (r.u8(2)? as usize * 512).min(data.len());
    Ok(RomMap {
        atom_ptr,
        mdt,
        mct,
        pp_off,
        vram_off,
        declared_bytes,
    })
}

/// Byte ranges that define the ROM's structure and boot behavior.
/// Any `--hex` write overlapping one of these is refused, and
/// [`verify_structural`] refuses if a byte here changes outside the
/// reported diffs (a deliberate, validated edit).
pub(super) fn layout_ranges(map: &RomMap, r: &Reader) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    // Everything before the ATOM header: signatures, the image size in
    // byte 2, the PCIR pointer at 0x18, the entry-point u16 at 0x48
    // that the loader uses to find the ATOM header, the build date /
    // OEM data block at 0x50+ and the vendor block right before the
    // ATOM header. A wrong byte here can prevent the BIOS from booting
    // or from finding any of the tables.
    out.push((0x00, map.atom_ptr));
    // ATOM_ROM_HEADER itself (usTableSize, revisions, subsystem IDs,
    // master table offsets at +0x1E/+0x20).
    if let Ok(sz) = r.u16(map.atom_ptr) {
        out.push((map.atom_ptr, map.atom_ptr.saturating_add(sz as usize)));
    }
    // Master List of Command Tables: the offset array that decides
    // where every command table lives.
    let mct_end = map
        .mct
        .saturating_add(header::master_cmd_table_layout_size());
    if map.mct > 0 && mct_end <= r.len() {
        out.push((map.mct, mct_end));
    }
    // Master Data Table: common header + all 35 offset entries.
    let mdt_end = map
        .mdt
        .saturating_add(header::master_data_table_layout_size());
    if map.mdt > 0 && mdt_end <= r.len() {
        out.push((map.mdt, mdt_end));
    }
    // PCI Data Structures (incl. their checksum byte).
    if let Ok(images) = crate::rom::pci::walk_pci_images(r) {
        for img in images {
            let end = img
                .pcir_offset
                .saturating_add(img.pcir_struct_length as usize)
                .min(r.len());
            out.push((img.pcir_offset, end));
        }
    }
    if let Some(pp_off) = map.pp_off {
        // ATOM_Tonga_POWERPLAYTABLE fixed header: all sub-table offsets
        // live at rel 35..63, so the first 65 bytes define the layout.
        out.push((pp_off, pp_off.saturating_add(65).min(r.len())));
        // Sub-table headers (revision/count bytes).
        if let Some(off) = crate::rom::locate::powerplay_sclk_table(r, pp_off) {
            out.push((off, off + 2));
        }
        if let Some(off) = crate::rom::locate::powerplay_mclk_table(r, pp_off) {
            out.push((off, off + 2));
        }
        if let Some(off) = crate::rom::locate::powerplay_vddc_lut(r, pp_off) {
            out.push((off, off + 2));
        }
        if let Some(off) = crate::rom::locate::powerplay_hard_limits(r, pp_off) {
            out.push((off, off + 2));
        }
        if let Some(off) = crate::rom::locate::powerplay_powertune(r, pp_off) {
            // Only the revision byte; the TDP u16 at +1 is data.
            out.push((off, off + 1));
        }
    }
    if let Some(vram_off) = map.vram_off {
        // VRAM_Info header: size, revisions, strap patch offset (+6).
        out.push((vram_off, vram_off.saturating_add(8).min(r.len())));
        // Strap register index table: which MC registers the straps set.
        if let Ok(patch_off) = r.u16(vram_off + 6)
            && patch_off != 0
        {
            let base = vram_off + patch_off as usize;
            if let Ok(idx_size) = r.u16(base) {
                let end = base.saturating_add(4 + idx_size as usize).min(r.len());
                out.push((base, end));
            }
        }
    }
    out
}

pub(super) fn overlaps_layout(
    map: &RomMap,
    r: &Reader,
    offset: usize,
    len: usize,
) -> Option<(usize, usize)> {
    layout_ranges(map, r)
        .into_iter()
        .find(|&(start, end)| offset < end && offset + len > start)
}

/// Ranges of the parsed structures (for the `--hex` overlap warning).
pub(super) fn structure_ranges(map: &RomMap, r: &Reader) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    // Command table bodies: --hex into the bytecode changes firmware
    // behavior, so warn (they are data, not layout).
    if map.mct > 0 {
        for idx in 0..header::COMMAND_TABLE_NAMES.len() {
            if let Ok(off) = r.u16(map.mct + 4 + idx * 2)
                && off != 0
                && let Ok(sz) = r.u16(off as usize)
                && sz > 0
            {
                out.push((off as usize, off as usize + sz as usize));
            }
        }
    }
    if let Some(pp_off) = map.pp_off {
        if let Ok(sz) = r.u16(pp_off) {
            out.push((pp_off, pp_off + sz as usize));
        }
        if let Some(off) = crate::rom::locate::powerplay_sclk_table(r, pp_off)
            && let (Ok(rev), Ok(n)) = (r.u8(off), r.u8(off + 1))
        {
            let esz = if rev >= 1 { 15 } else { 11 };
            out.push((off, off + 2 + n as usize * esz));
        }
        if let Some(off) = crate::rom::locate::powerplay_mclk_table(r, pp_off)
            && let Ok(n) = r.u8(off + 1)
        {
            out.push((off, off + 2 + n as usize * 13));
        }
        if let Some(off) = crate::rom::locate::powerplay_vddc_lut(r, pp_off)
            && let Ok(n) = r.u8(off + 1)
        {
            out.push((off, off + 2 + n as usize * 8));
        }
        if let Some(off) = crate::rom::locate::powerplay_powertune(r, pp_off) {
            out.push((off, off + 48));
        }
        if let Some(off) = crate::rom::locate::powerplay_hard_limits(r, pp_off)
            && let Ok(n) = r.u8(off + 1)
        {
            out.push((off, off + 2 + n as usize * 14));
        }
    }
    if let Some(vram_off) = map.vram_off {
        if let Ok(sz) = r.u16(vram_off) {
            out.push((vram_off, vram_off + sz as usize));
        }
        if let Some((data_start, block_size)) = crate::rom::locate::strap_region(r, vram_off) {
            let n = super::limits::strap_count(r, data_start, block_size);
            out.push((data_start, data_start + n * block_size));
        }
    }
    out
}

pub(super) fn structure_contains(map: &RomMap, r: &Reader, offset: usize, len: usize) -> bool {
    structure_ranges(map, r)
        .iter()
        .any(|&(start, end)| offset < end && offset + len > start)
}
