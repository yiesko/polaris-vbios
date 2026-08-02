//! Decodes the bytecode of ATOM command tables (the kernel's atom.c
//! interpreter semantics, decoding side only). Instruction layout and
//! opcode table live in [`opcodes`].

mod opcodes;

use std::collections::HashMap;

use anyhow::{Result, bail};

use super::header::{COMMAND_TABLE_NAMES, MASTER_TABLE_NAMES};
use super::reader::Reader;
use opcodes::{
    ARG_FB, ARG_ID, ARG_IMM, ARG_MC, ARG_PLL, ARG_PS, ARG_REG, ARG_WS, CASE_END, CASE_MAGIC,
    MAX_INSTRUCTIONS, OPCODE_NAMES, dst_space, ws_name,
};

/// One decoded instruction of a command table.
#[derive(Debug, Clone)]
pub struct DisasmLine {
    /// Byte offset relative to the table start (code begins at +6).
    pub addr: u16,
    pub text: String,
}

/// A disassembled ATOM command table.
#[derive(Debug, Clone)]
pub struct TableDisasm {
    pub index: usize,
    pub name: String,
    /// ROM offset of the table start.
    pub offset: usize,
    pub size: u16,
    pub fmt_rev: u8,
    pub cont_rev: u8,
    /// Workspace size in dwords (WS) and param stack size in bytes (PS).
    pub ws_size: u8,
    pub ps_size: u8,
    pub lines: Vec<DisasmLine>,
}

struct Ctx<'a> {
    r: &'a Reader<'a>,
    /// Command table start (absolute ROM offset); jump targets and
    /// ID blocks are relative to it.
    start: usize,
    /// First byte past the table's declared code region; no decode may
    /// read at or beyond it.
    end: usize,
    reg_block: u16,
    data_block: u16,
    reg_names: Option<&'a HashMap<u16, String>>,
}

/// Reads a source/destination argument described by `attr` at `*p`,
/// returning its rendered text and its byte length.
fn read_arg(ctx: &Ctx, attr: u8, p: &mut usize) -> Result<(String, usize)> {
    let space = attr & 7;
    let align = (attr >> 3) & 7;
    let off = *p;
    match space {
        ARG_REG => {
            let idx = ctx.r.u16(off)?;
            *p += 2;
            let abs = idx.wrapping_add(ctx.reg_block);
            let name = ctx
                .reg_names
                .and_then(|m| m.get(&abs))
                .map(|n| format!(" ({n})"))
                .unwrap_or_default();
            let base = if ctx.reg_block != 0 {
                format!("+{b:04X}", b = ctx.reg_block)
            } else {
                String::new()
            };
            Ok((format!("REG[0x{idx:04X}{base}]{name}"), 2))
        }
        ARG_PS => {
            let idx = ctx.r.u8(off)?;
            *p += 1;
            Ok((format!("PS[0x{idx:02X}]"), 1))
        }
        ARG_WS => {
            let idx = ctx.r.u8(off)?;
            *p += 1;
            let name = ws_name(idx).map(|n| format!(" ({n})")).unwrap_or_default();
            Ok((format!("WS[0x{idx:02X}]{name}"), 1))
        }
        ARG_FB => {
            let idx = ctx.r.u8(off)?;
            *p += 1;
            Ok((format!("FB[0x{idx:02X}]"), 1))
        }
        ARG_ID => {
            let idx = ctx.r.u16(off)?;
            *p += 2;
            let base = if ctx.data_block != 0 {
                format!("+{b:04X}", b = ctx.data_block)
            } else {
                String::new()
            };
            Ok((format!("ID[0x{idx:04X}{base}]"), 2))
        }
        ARG_IMM => read_direct(ctx, align, p),
        ARG_PLL => {
            let idx = ctx.r.u8(off)?;
            *p += 1;
            Ok((format!("PLL[0x{idx:02X}]"), 1))
        }
        ARG_MC => {
            let idx = ctx.r.u8(off)?;
            *p += 1;
            Ok((format!("MC[0x{idx:02X}]"), 1))
        }
        _ => bail!("bad argument space {space}"),
    }
}

/// Reads a direct (immediate) value: 4/2/1 bytes per align bits.
fn read_direct(ctx: &Ctx, align: u8, p: &mut usize) -> Result<(String, usize)> {
    let off = *p;
    let (text, len) = match align {
        0 => (format!("0x{:08X}", ctx.r.u32(off)?), 4),
        1..=3 => (format!("0x{:04X}", ctx.r.u16(off)?), 2),
        _ => (format!("0x{:02X}", ctx.r.u8(off)?), 1),
    };
    *p += len;
    Ok((text, len))
}

/// Decodes a `attr, dst, src` instruction; the dst space comes from the
/// opcode, the src from the attr byte. Returns the rendered text and the
/// instruction length in bytes (including the attr byte).
fn binary_args(ctx: &Ctx, op: u8, p: &mut usize) -> Result<(String, usize)> {
    let attr = ctx.r.u8(*p)?;
    *p += 1;
    let dst_align = (attr >> 3) & 7;
    let (dst, dl) = read_arg(ctx, dst_space(op) | (dst_align << 3), p)?;
    let (src, sl) = read_arg(ctx, attr, p)?;
    Ok((format!("{dst}, {src}"), dl + sl + 1))
}

/// Decodes one instruction at `*p`, appending it to `out`. Returns the
/// instruction length; `None` terminates the stream (EOT, or an opcode the
/// interpreter treats as end-of-table: 0x00 and anything >= 0x7B).
fn decode_instruction(
    ctx: &Ctx,
    p: &mut usize,
    out: &mut Vec<DisasmLine>,
) -> Result<Option<usize>> {
    let op = ctx.r.u8(*p)?;
    // 0x7A ("CTB_DS" in atom-names.h) has no interpreter entry in the
    // kernel's opcode_table; stop decoding like any out-of-range opcode.
    if op == 0 || op >= 0x7A {
        let addr = (*p - ctx.start - 6) as u16;
        if op == 0 {
            out.push(DisasmLine {
                addr,
                text: "; end of table (RESERVED 0x00)".to_string(),
            });
        } else {
            out.push(DisasmLine {
                addr,
                text: format!("; end of table (opcode 0x{op:02X}, interpreter stops)"),
            });
        }
        return Ok(None);
    }
    *p += 1;
    let name = OPCODE_NAMES[op as usize];
    let (args, len) = match op {
        0x01..=0x12 | 0x1F..=0x36 | 0x3C..=0x41 | 0x4A..=0x4F | 0x67..=0x6C => {
            // MOVE/AND/OR/MUL/DIV/ADD/SUB/COMPARE/TEST/XOR
            binary_args(ctx, op, p)?
        }
        0x6D..=0x78 => {
            // SHL/SHR families (kernel atom_op_shl/atom_op_shr): attr,
            // dst, then a full src argument for the shift amount.
            binary_args(ctx, op, p)?
        }
        0x13..=0x1E => {
            // SHIFT_LEFT/SHIFT_RIGHT (kernel atom_op_shift_left/right):
            // attr, dst, u8 shift.
            let attr = ctx.r.u8(*p)?;
            *p += 1;
            let dst_align = (attr >> 3) & 7;
            let (dst, dl) = read_arg(ctx, dst_space(op) | (dst_align << 3), p)?;
            let shift = ctx.r.u8(*p)?;
            *p += 1;
            (format!("{dst}, 0x{shift:02X}"), dl + 2)
        }
        0x37 => {
            // SET_ATI_PORT: u16 port (0 = MM, else IIO method)
            let port = ctx.r.u16(*p)?;
            *p += 2;
            let what = match port {
                0 => "MM".to_string(),
                1 => "IIO (PLL)".to_string(),
                2 => "IIO (MC)".to_string(),
                3 => "IIO (PCIE)".to_string(),
                4 => "IIO (PCIE PORT)".to_string(),
                n => format!("IIO (method {n})"),
            };
            (what, 2)
        }
        0x38 | 0x39 => {
            // SET_PCI_PORT / SET_SYS_IO_PORT: 1 byte
            let port = ctx.r.u8(*p)?;
            *p += 1;
            (format!("port {port}"), 1)
        }
        0x3A => {
            let base = ctx.r.u16(*p)?;
            *p += 2;
            (format!("0x{base:04X}"), 2)
        }
        0x3B => {
            // SET_FB_BASE: attr, src
            let attr = ctx.r.u8(*p)?;
            *p += 1;
            let (src, sl) = read_arg(ctx, attr, p)?;
            (src, sl + 1)
        }
        0x42 => {
            // SWITCH: attr, src, then (magic u8 0x63, imm case, target u16)*
            // terminated by u16 0x5A5A.
            let attr = ctx.r.u8(*p)?;
            *p += 1;
            let (src, sl) = read_arg(ctx, attr, p)?;
            let align = (attr >> 3) & 7;
            let mut cases = Vec::new();
            let mut len = sl + 1;
            loop {
                if *p + 2 > ctx.end {
                    bail!(
                        "unterminated SWITCH case list at 0x{:X} (no 0x5A5A terminator before the end of the table)",
                        *p - ctx.start - 6
                    );
                }
                if ctx.r.u16(*p)? == CASE_END {
                    *p += 2;
                    len += 2;
                    break;
                }
                if ctx.r.u8(*p)? != CASE_MAGIC {
                    bail!("bad SWITCH case marker 0x{:04X}", ctx.r.u16(*p)?);
                }
                *p += 1;
                len += 1;
                let (val, vl) = read_arg(ctx, ARG_IMM | (align << 3), p)?;
                let target = ctx.r.u16(*p)?;
                cases.push(format!("{val} -> +0x{target:04X}"));
                *p += 2;
                len += vl + 2;
            }
            (format!("{src} ({})", cases.join(", ")), len)
        }
        0x43..=0x49 => {
            // JUMP variants: u16 target relative to the table start
            let target = ctx.r.u16(*p)?;
            *p += 2;
            (format!("+0x{target:04X}"), 2)
        }
        0x50 => {
            let ms = ctx.r.u8(*p)?;
            *p += 1;
            (format!("{ms} ms"), 1)
        }
        0x51 => {
            let us = ctx.r.u8(*p)?;
            *p += 1;
            (format!("{us} us"), 1)
        }
        0x52 => {
            // CALL_TABLE: u8 command table index
            let idx = ctx.r.u8(*p)? as usize;
            *p += 1;
            let table = COMMAND_TABLE_NAMES.get(idx).copied().unwrap_or("(unknown)");
            (format!("{idx} ({table})"), 1)
        }
        0x53 | 0x64 | 0x65 | 0x79 => {
            // REPEAT, SAVE_REG, RESTORE_REG, DEBUG: the kernel
            // interpreter does not implement them (they consume no
            // operand bytes); render them as such.
            (String::from("(unimplemented)"), 0)
        }
        0x54..=0x59 => {
            // CLEAR: attr, dst
            let attr = ctx.r.u8(*p)?;
            *p += 1;
            let dst_align = (attr >> 3) & 7;
            let (dst, dl) = read_arg(ctx, dst_space(op) | (dst_align << 3), p)?;
            (dst, dl + 1)
        }
        0x5A | 0x63 => {
            // NOP / BEEP: no operands
            (String::new(), 0)
        }
        0x5B => {
            // EOT
            return Ok(None);
        }
        0x5C..=0x61 => {
            // MASK: attr, dst, direct mask, src
            let attr = ctx.r.u8(*p)?;
            *p += 1;
            let align = (attr >> 3) & 7;
            let (dst, dl) = read_arg(ctx, dst_space(op) | (align << 3), p)?;
            let (mask, ml) = read_direct(ctx, align, p)?;
            let (src, sl) = read_arg(ctx, attr, p)?;
            (format!("{dst}, mask {mask}, {src}"), dl + ml + sl + 1)
        }
        0x62 => {
            // POST_CARD: u8 card id
            let val = ctx.r.u8(*p)?;
            *p += 1;
            (format!("0x{val:02X}"), 1)
        }
        0x66 => {
            // SET_DATA_BLOCK: u8 index
            let idx = ctx.r.u8(*p)?;
            *p += 1;
            let table = match idx {
                0 => "0 (reset)".to_string(),
                255 => "255 (this table)".to_string(),
                n => {
                    let name = MASTER_TABLE_NAMES
                        .get(n as usize)
                        .copied()
                        .unwrap_or("(unknown)");
                    format!("{n} ({name})")
                }
            };
            (table, 1)
        }
        _ => bail!("unhandled opcode 0x{op:02X}"),
    };
    let addr = (*p - ctx.start - 6 - len) as u16;
    out.push(DisasmLine {
        addr,
        text: format!("{name:<18} {args}"),
    });
    Ok(Some(len))
}

/// Disassembles the command tables present in the master command table,
/// or only `table_filter` (by index or name). The byte stream is decoded
/// linearly from the code start (+6) until EOT (0x5B) or the table end.
pub fn disasm_command_tables(
    r: &Reader,
    master_cmd_offset: usize,
    table_filter: Option<&str>,
    reg_names: Option<&HashMap<u16, String>>,
) -> Result<Vec<TableDisasm>> {
    if master_cmd_offset == 0 {
        return Ok(Vec::new());
    }
    let filter_index = match table_filter {
        Some(f) => {
            if let Ok(idx) = f.parse::<usize>() {
                Some(idx)
            } else {
                let idx = COMMAND_TABLE_NAMES
                    .iter()
                    .position(|n| *n == f)
                    .ok_or_else(|| anyhow::anyhow!("no command table named '{f}'"))?;
                Some(idx)
            }
        }
        None => None,
    };

    let mut out = Vec::new();
    for (idx, name) in COMMAND_TABLE_NAMES.iter().enumerate() {
        if let Some(want) = filter_index
            && idx != want
        {
            continue;
        }
        let off = r.u16(master_cmd_offset + 4 + idx * 2)? as usize;
        if off == 0 {
            continue;
        }
        let size = r.u16(off)?;
        let fmt_rev = r.u8(off + 2)?;
        let cont_rev = r.u8(off + 3)?;
        let ws_size = r.u8(off + 4)?;
        let ps_size = r.u8(off + 5)?;

        let ctx = Ctx {
            r,
            start: off,
            end: off + size as usize,
            reg_block: 0,
            data_block: 0,
            reg_names,
        };
        let mut lines = Vec::new();
        let code_start = off + 6;
        let code_end = ctx.end;
        let mut p = code_start;
        let mut decoded = 0usize;
        loop {
            if p >= code_end {
                bail!("table '{name}' ran off the end (size {size}) without EOT");
            }
            match decode_instruction(&ctx, &mut p, &mut lines)? {
                // Zero-operand opcodes (NOP, DEBUG, BEEP, SAVE_REG,
                // RESTORE_REG, REPEAT) are valid: the opcode byte itself
                // was consumed, so each iteration advances and the
                // instruction cap bounds the loop.
                Some(0) => {}
                Some(_) => {}
                None => break,
            }
            decoded += 1;
            if decoded > MAX_INSTRUCTIONS {
                bail!("table '{name}' exceeds {MAX_INSTRUCTIONS} instructions (corrupt table?)");
            }
        }
        out.push(TableDisasm {
            index: idx,
            name: (*name).to_string(),
            offset: off,
            size,
            fmt_rev,
            cont_rev,
            ws_size,
            ps_size,
            lines,
        });
    }
    Ok(out)
}
