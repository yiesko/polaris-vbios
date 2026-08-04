use std::collections::BTreeMap;
use std::collections::BTreeSet;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct VramModule {
    pub index: usize,
    pub part_number: String,
    pub memory_size_mb: u16,
    pub memory_type_raw: u8,
    pub memory_type_name: String,
    pub channel_num: u8,
    pub vendor_id_raw: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryStrap {
    pub clock_mhz: f64,
    pub effective_gbps: f64,
    pub mem_block_id: u8,
    pub values: Vec<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VramInfo {
    pub struct_size: u16,
    pub fmt_rev: u8,
    pub cont_rev: u8,
    pub num_modules: u8,
    pub modules: Vec<VramModule>,
    pub strap_reg_indices: Vec<u16>,
    pub straps: Vec<MemoryStrap>,
    /// MC ucode (microcode) version from `ATOM_MC_INIT_PARAM_TABLE_V2_1`.
    pub mcu_code_version: Option<u32>,
    pub mcu_code_rom_start_addr: Option<u32>,
    pub mcu_code_length: Option<u32>,
}

/// Converts a strap clock in MHz to a rounded integer key suitable for
/// cross-ROM matching.
pub fn strap_clock_key(mhz: f64) -> i64 {
    mhz.round() as i64
}

/// Extension trait with convenience methods on `&[MemoryStrap]`.
pub trait StrapSliceExt {
    fn max_clock_mhz(&self) -> f64;
    fn all_clock_keys(&self) -> BTreeSet<i64>;
    fn find_by_clock_key(&self, clk: i64) -> Option<&MemoryStrap>;
    fn group_by_block(&self) -> BTreeMap<u8, Vec<&MemoryStrap>>;
}

impl StrapSliceExt for [MemoryStrap] {
    /// Highest clock across all straps, or `0.0` if empty.
    fn max_clock_mhz(&self) -> f64 {
        self.iter().map(|s| s.clock_mhz).fold(0.0, f64::max)
    }

    /// All distinct clock keys present in these straps.
    fn all_clock_keys(&self) -> BTreeSet<i64> {
        self.iter().map(|s| strap_clock_key(s.clock_mhz)).collect()
    }

    /// Finds the first strap whose rounded clock matches `clk`.
    fn find_by_clock_key(&self, clk: i64) -> Option<&MemoryStrap> {
        self.iter().find(|s| strap_clock_key(s.clock_mhz) == clk)
    }

    /// Groups straps by their memory block id.
    fn group_by_block(&self) -> BTreeMap<u8, Vec<&MemoryStrap>> {
        let mut map: BTreeMap<u8, Vec<&MemoryStrap>> = BTreeMap::new();
        for s in self {
            map.entry(s.mem_block_id).or_default().push(s);
        }
        map
    }
}
