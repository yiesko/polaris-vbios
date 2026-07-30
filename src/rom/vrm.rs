use anyhow::Result;

use super::reader::Reader;
use super::types::{EvvEntry, VoltageObject, VoltageObjectDetail, VrmInfo};

fn voltage_type_name(t: u8) -> String {
    match t {
        1 => "VDDC".to_string(),
        2 => "MVDDC".to_string(),
        3 => "MVDDQ".to_string(),
        4 => "VDDCI".to_string(),
        5 => "VDDGFX".to_string(),
        6 => "PCC".to_string(),
        7 => "MVPP".to_string(),
        8 => "LEDDPM".to_string(),
        9 => "PCC_MVDD".to_string(),
        10 => "PCIE_VDDC".to_string(),
        11 => "PCIE_VDDR".to_string(),
        0x11..=0x1A => format!("GENERIC_I2C_{}", t - 0x10),
        other => format!("unknown (0x{other:02X})"),
    }
}

fn voltage_mode_name(m: u8) -> &'static str {
    match m {
        0 => "GPIO LUT",
        3 => "Init via I2C (regulator)",
        4 => "Phase LUT (GPIO)",
        7 => "SVID2 (digital)",
        8 => "EVV (leakage-based adaptive voltage)",
        0x10 => "Leakage LUT (powerboost)",
        0x11 => "Leakage LUT (high state)",
        0x12 => "Leakage LUT (high1 state)",
        _ => "unknown",
    }
}

/// Voltage regulator circuit names - come verbatim from the public
/// `atombios.h` comments (not community reverse engineering, it is
/// the official `amdgpu` driver header).
fn voltage_control_id_name(id: u8) -> Option<&'static str> {
    Some(match id {
        0x01 => "LM64",
        0x02 => "DAC",
        0x03 => "VT116xM",
        0x04 => "DS4402",
        0x05 => "uP6266",
        0x06 => "Scorpio",
        0x07 => "VT1556M",
        0x08 => "CHL822x",
        0x09 => "VT1586M",
        0x0A => "uP1637",
        0x0B => "CHL8214",
        0x0C => "uP1801",
        0x0D => "ST6788A",
        0x0E => "CHL/IR3564 (SVI2)",
        0x0F => "AD527x",
        0x10 => "NCP81022",
        0x11 => "LTC2635",
        0x12 => "NCP4208",
        0x13 => "IR35xx",
        0x14 => "RT9403",
        0x40 => "Generic I2C",
        _ => return None,
    })
}

fn parse_detail(r: &Reader, p: usize, mode: u8, obj_end: usize) -> Result<VoltageObjectDetail> {
    Ok(match mode {
        0 | 4 => {
            let gpio_cntl_id = r.u8(p + 4)?;
            let entry_num = (r.u8(p + 5)? as usize).min(32);
            let phase_delay_us = r.u8(p + 6)?;
            let gpio_mask = r.u32(p + 8)?;
            let mut lut_mv = Vec::with_capacity(entry_num);
            let mut q = p + 12;
            for _ in 0..entry_num {
                if q + 6 > obj_end {
                    break;
                }
                let mv = r.u16(q + 4)?;
                lut_mv.push(mv);
                q += 6;
            }
            VoltageObjectDetail::GpioLut {
                gpio_cntl_id,
                phase_delay_us,
                gpio_mask,
                lut_mv,
            }
        }
        3 => {
            let regulator_id = r.u8(p + 4)?;
            let i2c_line = r.u8(p + 5)?;
            let i2c_address = r.u8(p + 6)?;
            // The list lives inside the declared object size - without
            // this limit, reading past the object end bleeds into the
            // NEXT object's bytes (this was the actual bug found when
            // validating against a real ROM: without the limit,
            // absurd values from the following object "leaked out").
            let mut init_pairs = Vec::new();
            let mut q = p + 12;
            while q + 4 <= obj_end {
                let code = r.u16(q)?;
                if code == 0xFFFF || code & 0xFF == 0xFF {
                    break;
                }
                init_pairs.push((code, r.u16(q + 2)?));
                q += 4;
            }
            VoltageObjectDetail::I2cInitSeq {
                regulator_id,
                regulator_name: voltage_control_id_name(regulator_id).map(str::to_string),
                i2c_line,
                i2c_address,
                init_pairs,
            }
        }
        7 => {
            let load_line_psi_raw = r.u16(p + 4)?;
            let svd_gpio_id = r.u8(p + 6)?;
            let svc_gpio_id = r.u8(p + 7)?;
            VoltageObjectDetail::Svid2 {
                svd_gpio_id,
                svc_gpio_id,
                load_line_psi_raw,
            }
        }
        8 => {
            let mut entries = Vec::with_capacity(8);
            let mut q = p + 4;
            for _ in 0..8 {
                if q + 8 > obj_end {
                    break;
                }
                let sclk = r.u32(q)?;
                let voffset_raw = r.u16(q + 4)?;
                let v_adj_offset_mv = if voffset_raw > 32768 {
                    voffset_raw as i32 - 65536
                } else {
                    voffset_raw as i32
                };
                let dpm_v_index = r.u8(q + 6)?;
                let dpm_state = r.u8(q + 7)?;
                entries.push(EvvEntry {
                    dpm_sclk_mhz: sclk as f64 / 100.0,
                    v_adj_offset_mv,
                    dpm_v_index,
                    dpm_state,
                });
                q += 8;
            }
            VoltageObjectDetail::Evv { entries }
        }
        0x10..=0x12 => VoltageObjectDetail::LeakageLut {
            entries_count: r.u8(p + 5)?,
        },
        _ => VoltageObjectDetail::Unknown,
    })
}

/// Parses `ATOM_VOLTAGE_OBJECT_INFO_V3_1` - the table that describes how
/// each voltage rail (VDDC/VDDCI/VDDGFX/MVDD...) is controlled: by GPIO,
/// by an I2C regulator (sometimes with the chip identified), or
/// digitally via SVID2. Each object has a variable size (`usSize`), so
/// the list is traversed by adding this size at each step - not a
/// fixed-size array.
pub fn parse_voltage_object_info(r: &Reader, off: usize) -> Result<VrmInfo> {
    let struct_size = r.u16(off)? as usize;
    let fmt_rev = r.u8(off + 2)?;
    let cont_rev = r.u8(off + 3)?;
    let recognized_format = fmt_rev == 3 && cont_rev == 1;

    let mut objects = Vec::new();
    if recognized_format {
        let end = (off + struct_size).min(r.len());
        let mut p = off + 4;
        let mut count = 0;
        while p + 4 <= end && count < 8 {
            let voltage_type_raw = r.u8(p)?;
            let mode_raw = r.u8(p + 1)?;
            let size = r.u16(p + 2)?;
            if size < 4 {
                break;
            }
            let obj_end = (p + size as usize).min(end);
            let detail = parse_detail(r, p, mode_raw, obj_end)?;
            objects.push(VoltageObject {
                voltage_type_raw,
                voltage_type_name: voltage_type_name(voltage_type_raw),
                mode_raw,
                mode_name: voltage_mode_name(mode_raw).to_string(),
                size,
                detail,
            });
            p += size as usize;
            count += 1;
        }
    }

    Ok(VrmInfo {
        fmt_rev,
        cont_rev,
        recognized_format,
        objects,
    })
}
