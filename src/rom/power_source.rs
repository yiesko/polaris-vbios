use anyhow::Result;

use super::reader::Reader;
use super::types::{PowerSourceInfo, PowerSourceObject};

/// `ucPwrSrcId` values (atombios.h line 5698).
fn source_name(id: u8) -> &'static str {
    match id {
        0x00 => "PCIe slot",
        0x01 => "6-pin connector #1",
        0x02 => "8-pin connector #1",
        0x04 => "6-pin connector #2",
        0x08 => "8-pin connector #2",
        _ => "unknown",
    }
}

/// `ucPwrSensorType` values (atombios.h line 5705).
fn sensor_type_name(t: u8) -> &'static str {
    match t {
        0x00 => "always present",
        0x01 => "GPIO",
        0x02 => "I2C",
        _ => "unknown",
    }
}

/// Parses `ATOM_POWER_SOURCE_INFO` (PowerSourceInfo table, atombios.h
/// line 5690). After the common header and the 16-byte power behavior
/// block comes a variable number of 12-byte `ATOM_POWER_SOURCE_OBJECT`
/// entries; the count is derived from the declared structure size.
pub fn parse_power_source_info(r: &Reader, off: usize) -> Result<PowerSourceInfo> {
    let struct_size = r.u16(off)?;
    let fmt_rev = r.u8(off + 2)?;
    let cont_rev = r.u8(off + 3)?;

    let obj_base = off + 20;
    let avail = struct_size.saturating_sub(20) / 12;
    let mut objects = Vec::with_capacity(avail as usize);
    for i in 0..avail {
        let p = obj_base + i as usize * 12;
        let src_id = r.u8(p)?;
        let sensor_type = r.u8(p + 1)?;
        objects.push(PowerSourceObject {
            index: i as usize,
            source_id_raw: src_id,
            source_name: source_name(src_id).to_string(),
            sensor_type_raw: sensor_type,
            sensor_type_name: sensor_type_name(sensor_type).to_string(),
            sensor_id: r.u8(p + 2)?,
            sensor_slave_addr: r.u8(p + 3)?,
            sensor_reg_index: r.u8(p + 4)?,
            sensor_reg_bit_mask: r.u8(p + 5)?,
            sensor_active_state: r.u8(p + 6)?,
            sensed_power_w: r.u16(p + 10)?,
        });
    }

    Ok(PowerSourceInfo {
        struct_size,
        fmt_rev,
        cont_rev,
        objects,
    })
}
