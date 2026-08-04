use anyhow::Result;

use super::parse_optional;
use super::reader::Reader;
use super::types::*;

fn subtable_vec<T>(
    r: &Reader<'_>,
    base: usize,
    offset: usize,
    label: &str,
    parse_fn: fn(&Reader<'_>, usize) -> Result<Vec<T>>,
) -> Result<Vec<T>> {
    if offset != 0 {
        parse_fn(r, base + offset)
            .map_err(|e| anyhow::anyhow!("failed to parse {label} subtable: {e}"))
    } else {
        Ok(Vec::new())
    }
}

fn thermal_controller_name(kind: u8) -> &'static str {
    match kind {
        0 => "None",
        17 => "LM96163",
        21 => "Tonga",
        22 => "Fiji",
        23 => "Polaris10",
        24 => "Vega10",
        _ => "unknown",
    }
}

/// Converts a 16-bit value interpreted as a signed offset
/// (the pattern used in the usXxxOffset fields of these tables).
pub fn signed16(v: u16) -> i32 {
    v as i16 as i32
}

/// Decodes a bitfield value against a table of (bitmask, label) pairs.
fn decode_bitfield<T>(value: T, bits: &[(T, &str)]) -> Vec<String>
where
    T: Copy + std::ops::BitAnd<Output = T> + PartialEq + From<u8>,
{
    bits.iter()
        .filter(|(bit, _)| value & *bit != T::from(0))
        .map(|(_, name)| name.to_string())
        .collect()
}

fn decode_platform_caps(caps: u32) -> Vec<String> {
    decode_bitfield(
        caps,
        &[
            (0x1, "VDDGFX_CONTROL (separate VDDGFX rail)"),
            (0x2, "POWERPLAY (mobile/CCC power page)"),
            (0x4, "SBIOSPOWERSOURCE"),
            (0x8, "DISABLE_VOLTAGE_ISLAND"),
            (0x20, "HARDWAREDC"),
            (0x1000, "MVDD_CONTROL (dynamic MVDD)"),
            (0x8000, "VDDCI_CONTROL (dynamic VDDCI)"),
            (0x20000, "BACO"),
            (0x100000, "OUTPUT_THERMAL2GPIO17"),
            (0x1000000, "COMBINE_PCC_WITH_THERMAL_SIGNAL"),
            (0x2000000, "LOAD_POST_PRODUCTION_FIRMWARE"),
        ],
    )
}

/// Decodes the `usClassification` field of a Tonga/Polaris state.
/// Flag bits from the Linux kernel's pptable.h
/// (drivers/gpu/drm/radeon/pptable.h, "ATOM_PPLIB_NONCLOCK_INFO").
fn decode_classification(classification: u16) -> Vec<String> {
    let mut out = decode_bitfield(
        classification,
        &[
            (0x0008, "BOOT"),
            (0x0010, "THERMAL"),
            (0x0020, "LIMITEDPOWERSOURCE"),
            (0x0040, "REST"),
            (0x0080, "FORCED"),
            (0x0100, "3DPERFORMANCE"),
            (0x0200, "OVERDRIVETEMPLATE"),
            (0x0400, "UVDSTATE"),
            (0x0800, "3DLOW"),
            (0x1000, "ACPI"),
            (0x2000, "HD2STATE"),
            (0x4000, "HDSTATE"),
            (0x8000, "SDSTATE"),
        ],
    );
    // UI bits [2:0] (UI_NONE = 0 means "no UI state", so it is skipped).
    let ui = classification & 0x0007;
    if ui != 0 {
        let name = match ui {
            1 => "UI_BATTERY",
            3 => "UI_BALANCED",
            5 => "UI_PERFORMANCE",
            _ => "UI_RESERVED",
        };
        out.push(name.to_string());
    }
    out
}

fn parse_states(r: &Reader, off: usize) -> Result<Vec<StateEntry>> {
    let n = r.u8(off + 1)? as usize;
    let mut out = Vec::with_capacity(n);
    let mut p = off + 2;
    for _ in 0..n {
        let eh = r.u8(p)? as u16;
        let el = r.u8(p + 1)? as u16;
        let mh = r.u8(p + 2)? as u16;
        let ml = r.u8(p + 3)? as u16;
        let pgl = r.u8(p + 4)? as u16;
        let pgh = r.u8(p + 5)? as u16;
        let pll = r.u8(p + 6)? as u16;
        let plh = r.u8(p + 7)? as u16;
        let classification = r.u16(p + 8)?;
        let caps = r.u32(p + 10)?;
        out.push(StateEntry {
            engine_clock_index: (eh << 8) | el,
            memory_clock_index: (mh << 8) | ml,
            pcie_gen: (pgh << 8) | pgl,
            pcie_lane: (plh << 8) | pll,
            classification,
            classification_decoded: decode_classification(classification),
            caps,
        });
        p += 20;
    }
    Ok(out)
}

fn parse_thermal_controller(r: &Reader, off: usize) -> Result<ThermalController> {
    let rev = r.u8(off)?;
    let kind = r.u8(off + 1)?;
    Ok(ThermalController {
        rev,
        kind,
        kind_name: thermal_controller_name(kind).to_string(),
        i2c_line: r.u8(off + 2)?,
        i2c_addr: r.u8(off + 3)?,
        fan_params: r.u8(off + 4)?,
        fan_min_rpm_x100: r.u8(off + 5)?,
        fan_max_rpm_x100: r.u8(off + 6)?,
    })
}

fn parse_sclk_table(r: &Reader, off: usize) -> Result<Vec<SclkEntry>> {
    let rev = r.u8(off)?;
    let n = r.u8(off + 1)? as usize;
    let is_polaris = rev >= 1;
    let entry_size = if is_polaris { 15 } else { 11 };
    let mut out = Vec::with_capacity(n);
    let mut p = off + 2;
    for level in 0..n {
        let vdd_index = r.u8(p)?;
        let vddc_offset_mv = signed16(r.u16(p + 1)?);
        let sclk = r.u32(p + 3)?;
        let edc_current = r.u16(p + 7)?;
        let reliability_temp_c = r.u8(p + 9)?;
        out.push(SclkEntry {
            level,
            sclk_mhz: sclk as f64 / 100.0,
            vdd_index,
            vddc_offset_mv,
            edc_current,
            reliability_temp_c,
        });
        p += entry_size;
    }
    Ok(out)
}

fn parse_mclk_table(
    r: &Reader,
    off: usize,
    vddc_lut: &[VoltageLutEntry],
) -> Result<Vec<MclkEntry>> {
    let n = r.u8(off + 1)? as usize;
    let mut out = Vec::with_capacity(n);
    let mut p = off + 2;
    for level in 0..n {
        let vddc_index = r.u8(p)?;
        let vddci_mv = r.u16(p + 1)?;
        let vddgfx_offset_mv = signed16(r.u16(p + 3)?);
        let mvdd_mv = r.u16(p + 5)?;
        let mclk = r.u32(p + 7)?;
        let resolved = vddc_lut
            .get(vddc_index as usize)
            .filter(|e| e.valid)
            .map(|e| e.vdd_mv);
        out.push(MclkEntry {
            level,
            mclk_mhz: mclk as f64 / 100.0,
            vddc_index,
            vddc_resolved_mv: resolved,
            vddci_mv,
            mvdd_mv,
            vddgfx_offset_mv,
        });
        p += 13;
    }
    Ok(out)
}

fn parse_voltage_lut(r: &Reader, off: usize) -> Result<Vec<VoltageLutEntry>> {
    let n = r.u8(off + 1)? as usize;
    let mut out = Vec::with_capacity(n);
    let mut p = off + 2;
    for index in 0..n {
        let vdd_mv = r.u16(p)?;
        // "Placeholder" slots not used by the ROM appear as 0xFFxx
        // (>= 2000 mV is physically impossible for GPU VDDC, so we
        // use this as a signal that the slot is not a real voltage).
        let valid = vdd_mv < 2000;
        out.push(VoltageLutEntry {
            index,
            vdd_mv,
            valid,
        });
        p += 8;
    }
    Ok(out)
}

fn parse_mm_table(r: &Reader, off: usize) -> Result<Vec<MmEntry>> {
    let n = r.u8(off + 1)? as usize;
    let mut out = Vec::with_capacity(n);
    let mut p = off + 2;
    for _ in 0..n {
        let vddc_index = r.u8(p)?;
        let dclk = r.u32(p + 3)?;
        let vclk = r.u32(p + 7)?;
        let eclk = r.u32(p + 11)?;
        let samuclk = r.u32(p + 19)?;
        out.push(MmEntry {
            vddc_index,
            uvd_dclk_mhz: dclk as f64 / 100.0,
            uvd_vclk_mhz: vclk as f64 / 100.0,
            vce_eclk_mhz: eclk as f64 / 100.0,
            samu_clk_mhz: samuclk as f64 / 100.0,
        });
        p += 23;
    }
    Ok(out)
}

fn parse_powertune(r: &Reader, off: usize) -> Result<PowerTune> {
    let revid = r.u8(off)?;
    let p = off + 1;
    Ok(PowerTune {
        revid,
        tdp_w: r.u16(p)?,
        configurable_tdp_w: r.u16(p + 2)?,
        tdc_a: r.u16(p + 4)?,
        battery_power_limit_w: r.u16(p + 6)?,
        small_power_limit_w: r.u16(p + 8)?,
        max_power_delivery_limit_w: r.u16(p + 14)?,
        tjmax_c: r.u16(p + 16)?,
        software_shutdown_temp_c: r.u16(p + 22)?,
        temp_limit_hotspot_c: r.u16(p + 26)?,
        temp_limit_liquid1_c: r.u16(p + 28)?,
        temp_limit_liquid2_c: r.u16(p + 30)?,
        temp_limit_vr_vddc_c: r.u16(p + 32)?,
        temp_limit_vr_mvdd_c: r.u16(p + 34)?,
        temp_limit_plx_c: r.u16(p + 36)?,
        boost_power_limit_w: r.u16(p + 45)?,
    })
}

fn parse_fan_table(r: &Reader, off: usize) -> Result<FanTable> {
    let rev = r.u8(off)?;
    let p = off + 1;
    let t_hyst_c = r.u8(p)?;
    let t_min = r.u16(p + 1)?;
    let t_med = r.u16(p + 3)?;
    let t_high = r.u16(p + 5)?;
    let pwm_min = r.u16(p + 7)?;
    let pwm_med = r.u16(p + 9)?;
    let pwm_high = r.u16(p + 11)?;
    let t_max = r.u16(p + 13)?;
    let p2 = p + 15;
    let fan_control_mode = r.u8(p2)?;
    let fan_pwm_max_pct = r.u16(p2 + 1)?;
    let fan_rpm_max = r.u16(p2 + 5)?;
    let target_temperature_c = r.u8(p2 + 11)?;
    let minimum_pwm_limit_pct = r.u8(p2 + 12)?;
    let p4 = p2 + 13 + 14;
    let (zero_rpm_enable, fan_stop_temperature_c, fan_start_temperature_c) =
        match (r.u8(p4), r.u8(p4 + 1), r.u8(p4 + 2)) {
            (Ok(a), Ok(b), Ok(c)) => (a, b, c),
            _ => (0, 0, 0),
        };
    Ok(FanTable {
        rev,
        t_hyst_c,
        t_min_c: t_min as f64 / 100.0,
        t_med_c: t_med as f64 / 100.0,
        t_high_c: t_high as f64 / 100.0,
        t_max_c: t_max as f64 / 100.0,
        pwm_min_pct: pwm_min as f64 / 100.0,
        pwm_med_pct: pwm_med as f64 / 100.0,
        pwm_high_pct: pwm_high as f64 / 100.0,
        fan_control_mode,
        fan_pwm_max_pct,
        fan_rpm_max,
        target_temperature_c,
        minimum_pwm_limit_pct,
        zero_rpm_enable,
        fan_stop_temperature_c,
        fan_start_temperature_c,
    })
}

fn parse_pcie_table(r: &Reader, off: usize) -> Result<Vec<PcieEntry>> {
    let n = r.u8(off + 1)? as usize;
    let mut out = Vec::with_capacity(n);
    let mut p = off + 2;
    for _ in 0..n {
        out.push(PcieEntry {
            pcie_gen: r.u8(p)?,
            pcie_lane_width: r.u8(p + 1)?,
        });
        p += 8;
    }
    Ok(out)
}

/// `ATOM_Tonga_VCE_State_Table` - maps each "VCE state" (video encoder
/// usage profile) to which SCLK/MCLK DPM level the driver should use
/// while that state is active.
fn parse_vce_state_table(r: &Reader, off: usize) -> Result<Vec<VceStateEntry>> {
    let n = r.u8(off + 1)? as usize;
    let mut out = Vec::with_capacity(n);
    let mut p = off + 2;
    for index in 0..n {
        out.push(VceStateEntry {
            index,
            vce_clock_index: r.u8(p)?,
            flag: r.u8(p + 1)?,
            sclk_index: r.u8(p + 2)?,
            mclk_index: r.u8(p + 3)?,
        });
        p += 4;
    }
    Ok(out)
}

/// `ATOM_Tonga_Hard_Limit_Table` - the absolute clock and voltage
/// limits (not overridable by overdrive) that the driver must never
/// exceed, regardless of what other tables allow.
fn parse_hard_limit_table(r: &Reader, off: usize) -> Result<Vec<HardLimitEntry>> {
    let n = r.u8(off + 1)? as usize;
    let mut out = Vec::with_capacity(n);
    let mut p = off + 2;
    for _ in 0..n {
        let sclk = r.u32(p)?;
        let mclk = r.u32(p + 4)?;
        out.push(HardLimitEntry {
            sclk_limit_mhz: sclk as f64 / 100.0,
            mclk_limit_mhz: mclk as f64 / 100.0,
            vddc_limit_mv: r.u16(p + 8)?,
            vddci_limit_mv: r.u16(p + 10)?,
            vddgfx_limit_mv: r.u16(p + 12)?,
        });
        p += 14;
    }
    Ok(out)
}

/// Parses the `ATOM_Tonga_POWERPLAYTABLE` (format revision 7), used by
/// the entire Tonga/Fiji/Polaris family - including Polaris10/Polaris20
/// (RX 470/480/570/580). All sub-table offsets are relative to the
/// start of this table (`off`).
pub fn parse_powerplay(r: &Reader, off: usize) -> Result<PowerPlay> {
    let (struct_size_total, header_fmt_rev, header_cont_rev) = r.table_header(off)?;
    let table_revision = r.u8(off + 4)?;

    let platform_caps = r.u32(off + 19)?;
    let max_od_engine = r.u32(off + 23)?;
    let max_od_memory = r.u32(off + 27)?;
    let power_control_limit = r.u16(off + 31)?;

    let state_arr_off = r.u16(off + 35)? as usize;
    let fan_table_off = r.u16(off + 37)? as usize;
    let thermal_ctrl_off = r.u16(off + 39)? as usize;
    let mclk_dep_off = r.u16(off + 43)? as usize;
    let sclk_dep_off = r.u16(off + 45)? as usize;
    let vddc_lut_off = r.u16(off + 47)? as usize;
    let vddgfx_lut_off = r.u16(off + 49)? as usize;
    let mm_dep_off = r.u16(off + 51)? as usize;
    let vce_state_off = r.u16(off + 53)? as usize;
    let powertune_off = r.u16(off + 57)? as usize;
    let hardlimit_off = r.u16(off + 59)? as usize;
    let pcie_off = r.u16(off + 61)? as usize;
    let gpio_off = r.u16(off + 63)? as usize;

    let vddc_lut = subtable_vec(r, off, vddc_lut_off, "VDDC LUT", parse_voltage_lut)?;
    let vddgfx_lut = subtable_vec(r, off, vddgfx_lut_off, "VDDGFX LUT", parse_voltage_lut)?;

    Ok(PowerPlay {
        header_fmt_rev,
        header_cont_rev,
        table_revision,
        struct_size_total,
        platform_caps,
        platform_caps_decoded: decode_platform_caps(platform_caps),
        max_overdrive_engine_mhz: max_od_engine as f64 / 100.0,
        max_overdrive_memory_mhz: max_od_memory as f64 / 100.0,
        power_control_limit_pct: power_control_limit,
        states: subtable_vec(r, off, state_arr_off, "states", parse_states)?,
        thermal_controller: parse_optional(
            r,
            off,
            thermal_ctrl_off,
            "thermal controller",
            parse_thermal_controller,
        )?,
        sclk_table: subtable_vec(r, off, sclk_dep_off, "SCLK table", parse_sclk_table)?,
        mclk_table: if mclk_dep_off != 0 {
            parse_mclk_table(r, off + mclk_dep_off, &vddc_lut)?
        } else {
            Vec::new()
        },
        mm_table: subtable_vec(r, off, mm_dep_off, "multimedia table", parse_mm_table)?,
        powertune: parse_optional(r, off, powertune_off, "powertune", parse_powertune)?,
        fan_table: parse_optional(r, off, fan_table_off, "fan table", parse_fan_table)?,
        pcie_table: subtable_vec(r, off, pcie_off, "PCIe table", parse_pcie_table)?,
        vrhot_sclk_dpm_index: if gpio_off != 0 {
            Some(r.u8(off + gpio_off + 1)?)
        } else {
            None
        },
        vce_states: subtable_vec(
            r,
            off,
            vce_state_off,
            "VCE state table",
            parse_vce_state_table,
        )?,
        hard_limits: subtable_vec(
            r,
            off,
            hardlimit_off,
            "hard limit table",
            parse_hard_limit_table,
        )?,
        vddc_lut,
        vddgfx_lut,
    })
}
