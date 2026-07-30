/// Opcode names indexed by opcode byte, from the Linux kernel's
/// atom-names.h (`drivers/gpu/drm/radeon/atom-names.h`). The kernel
/// interpreter (atom.c) defines the exact byte layout of each
/// instruction; this module mirrors only the decoding side.
pub(super) const OPCODE_NAMES: [&str; 123] = [
    "RESERVED",
    "MOVE_REG",
    "MOVE_PS",
    "MOVE_WS",
    "MOVE_FB",
    "MOVE_PLL",
    "MOVE_MC",
    "AND_REG",
    "AND_PS",
    "AND_WS",
    "AND_FB",
    "AND_PLL",
    "AND_MC",
    "OR_REG",
    "OR_PS",
    "OR_WS",
    "OR_FB",
    "OR_PLL",
    "OR_MC",
    "SHIFT_LEFT_REG",
    "SHIFT_LEFT_PS",
    "SHIFT_LEFT_WS",
    "SHIFT_LEFT_FB",
    "SHIFT_LEFT_PLL",
    "SHIFT_LEFT_MC",
    "SHIFT_RIGHT_REG",
    "SHIFT_RIGHT_PS",
    "SHIFT_RIGHT_WS",
    "SHIFT_RIGHT_FB",
    "SHIFT_RIGHT_PLL",
    "SHIFT_RIGHT_MC",
    "MUL_REG",
    "MUL_PS",
    "MUL_WS",
    "MUL_FB",
    "MUL_PLL",
    "MUL_MC",
    "DIV_REG",
    "DIV_PS",
    "DIV_WS",
    "DIV_FB",
    "DIV_PLL",
    "DIV_MC",
    "ADD_REG",
    "ADD_PS",
    "ADD_WS",
    "ADD_FB",
    "ADD_PLL",
    "ADD_MC",
    "SUB_REG",
    "SUB_PS",
    "SUB_WS",
    "SUB_FB",
    "SUB_PLL",
    "SUB_MC",
    "SET_ATI_PORT",
    "SET_PCI_PORT",
    "SET_SYS_IO_PORT",
    "SET_REG_BLOCK",
    "SET_FB_BASE",
    "COMPARE_REG",
    "COMPARE_PS",
    "COMPARE_WS",
    "COMPARE_FB",
    "COMPARE_PLL",
    "COMPARE_MC",
    "SWITCH",
    "JUMP",
    "JUMP_EQUAL",
    "JUMP_BELOW",
    "JUMP_ABOVE",
    "JUMP_BELOW_OR_EQUAL",
    "JUMP_ABOVE_OR_EQUAL",
    "JUMP_NOT_EQUAL",
    "TEST_REG",
    "TEST_PS",
    "TEST_WS",
    "TEST_FB",
    "TEST_PLL",
    "TEST_MC",
    "DELAY_MILLISEC",
    "DELAY_MICROSEC",
    "CALL_TABLE",
    "REPEAT",
    "CLEAR_REG",
    "CLEAR_PS",
    "CLEAR_WS",
    "CLEAR_FB",
    "CLEAR_PLL",
    "CLEAR_MC",
    "NOP",
    "EOT",
    "MASK_REG",
    "MASK_PS",
    "MASK_WS",
    "MASK_FB",
    "MASK_PLL",
    "MASK_MC",
    "POST_CARD",
    "BEEP",
    "SAVE_REG",
    "RESTORE_REG",
    "SET_DATA_BLOCK",
    "XOR_REG",
    "XOR_PS",
    "XOR_WS",
    "XOR_FB",
    "XOR_PLL",
    "XOR_MC",
    "SHL_REG",
    "SHL_PS",
    "SHL_WS",
    "SHL_FB",
    "SHL_PLL",
    "SHL_MC",
    "SHR_REG",
    "SHR_PS",
    "SHR_WS",
    "SHR_FB",
    "SHR_PLL",
    "SHR_MC",
    "DEBUG",
    "CTB_DS",
];

pub(super) const ARG_REG: u8 = 0;
pub(super) const ARG_PS: u8 = 1;
pub(super) const ARG_WS: u8 = 2;
pub(super) const ARG_FB: u8 = 3;
pub(super) const ARG_ID: u8 = 4;
pub(super) const ARG_IMM: u8 = 5;
pub(super) const ARG_PLL: u8 = 6;
pub(super) const ARG_MC: u8 = 7;
pub(super) const CASE_MAGIC: u8 = 0x63;

pub(super) const CASE_END: u16 = 0x5A5A;

/// Upper bound on decoded instructions per table; protects against
/// decoder hangs on malformed input.
pub(super) const MAX_INSTRUCTIONS: usize = 1 << 20;

pub(super) const WS_QUOTIENT: u8 = 0x40;
pub(super) const WS_REMAINDER: u8 = 0x41;
pub(super) const WS_DATAPTR: u8 = 0x42;
pub(super) const WS_SHIFT: u8 = 0x43;
pub(super) const WS_OR_MASK: u8 = 0x44;
pub(super) const WS_AND_MASK: u8 = 0x45;
pub(super) const WS_FB_WINDOW: u8 = 0x46;
pub(super) const WS_ATTRIBUTES: u8 = 0x47;
pub(super) const WS_REGPTR: u8 = 0x48;

/// Special names for the workspace slots defined by the interpreter.
pub(super) fn ws_name(idx: u8) -> Option<&'static str> {
    match idx {
        WS_QUOTIENT => Some("QUOTIENT"),
        WS_REMAINDER => Some("REMAINDER"),
        WS_DATAPTR => Some("DATAPTR"),
        WS_SHIFT => Some("SHIFT"),
        WS_OR_MASK => Some("OR_MASK"),
        WS_AND_MASK => Some("AND_MASK"),
        WS_FB_WINDOW => Some("FB_WINDOW"),
        WS_ATTRIBUTES => Some("ATTRIBUTES"),
        WS_REGPTR => Some("REGPTR"),
        _ => None,
    }
}

/// First opcode of every 6-arg family (dst space = index into DST_SPACES).
/// Matches `opcode_table` in the kernel's atom.c: MOVE 0x01, AND 0x07,
/// OR 0x0D, SHIFT_LEFT 0x13, SHIFT_RIGHT 0x19, MUL 0x1F, DIV 0x25,
/// ADD 0x2B, SUB 0x31, COMPARE 0x3C, TEST 0x4A, CLEAR 0x54, MASK 0x5C,
/// XOR 0x67, SHL 0x6D, SHR 0x73.
const FAMILY_BASES: [u8; 16] = [
    0x01, 0x07, 0x0D, 0x13, 0x19, 0x1F, 0x25, 0x2B, 0x31, 0x3C, 0x4A, 0x54, 0x5C, 0x67, 0x6D, 0x73,
];

/// Argument space of a destination inside a 6-arg family, in opcode
/// order (REG=0, PS=1, WS=2, FB=3, PLL=6, MC=7 - ID/IMM are never
/// destinations), matching `opcode_table` in the kernel's atom.c.
const DST_SPACES: [u8; 6] = [ARG_REG, ARG_PS, ARG_WS, ARG_FB, ARG_PLL, ARG_MC];

/// Destination argument space of an opcode from one of the 6-arg families.
pub(super) fn dst_space(op: u8) -> u8 {
    let pos = FAMILY_BASES
        .iter()
        .find(|base| op >= **base && op < **base + 6)
        .map(|base| (op - base) % 6)
        .expect("dst_space called with a non-family opcode");
    DST_SPACES[pos as usize]
}
