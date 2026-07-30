use crate::render::sections::Section;
use crate::rom::types::ParsedRom;

pub fn is_csv_exportable(s: Section) -> bool {
    matches!(
        s,
        Section::Sclk
            | Section::Mclk
            | Section::Straps
            | Section::Multimedia
            | Section::Vram
            | Section::Pcie
    )
}

pub fn exportable_keys() -> Vec<&'static str> {
    Section::ALL
        .iter()
        .copied()
        .filter(|s| is_csv_exportable(*s))
        .map(|s| s.key())
        .collect()
}

pub fn export_csv(roms: &[&ParsedRom], section: Section) -> Result<String, String> {
    let mut w = csv::Writer::from_writer(Vec::new());
    match section {
        Section::Sclk => sclk_csv(&mut w, roms),
        Section::Mclk => mclk_csv(&mut w, roms),
        Section::Straps => straps_csv(&mut w, roms),
        Section::Multimedia => mm_csv(&mut w, roms),
        Section::Vram => vram_csv(&mut w, roms),
        Section::Pcie => pcie_csv(&mut w, roms),
        _ => {
            return Err(format!(
                "section '{}' is not exportable as CSV (it is structured/free-form content, not a flat table). Tabular sections: {}",
                section.key(),
                exportable_keys().join(", ")
            ));
        }
    }
    w.flush().map_err(|e| e.to_string())?;
    String::from_utf8(w.into_inner().map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

fn sclk_csv(w: &mut csv::Writer<Vec<u8>>, roms: &[&ParsedRom]) {
    w.write_record(["rom", "level", "sclk_mhz", "vdd_index", "vddc_offset_mv"])
        .ok();
    for r in roms {
        for e in &r.powerplay.sclk_table {
            w.write_record([
                &r.file_name,
                &e.level.to_string(),
                &format!("{:.0}", e.sclk_mhz),
                &e.vdd_index.to_string(),
                &e.vddc_offset_mv.to_string(),
            ])
            .ok();
        }
    }
}

fn mclk_csv(w: &mut csv::Writer<Vec<u8>>, roms: &[&ParsedRom]) {
    w.write_record([
        "rom",
        "level",
        "mclk_mhz",
        "vddc_resolved_mv",
        "vddci_mv",
        "mvdd_mv",
    ])
    .ok();
    for r in roms {
        for e in &r.powerplay.mclk_table {
            w.write_record([
                &r.file_name,
                &e.level.to_string(),
                &format!("{:.0}", e.mclk_mhz),
                &e.vddc_resolved_mv
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                &e.vddci_mv.to_string(),
                &e.mvdd_mv.to_string(),
            ])
            .ok();
        }
    }
}

fn mm_csv(w: &mut csv::Writer<Vec<u8>>, roms: &[&ParsedRom]) {
    w.write_record([
        "rom",
        "level",
        "uvd_dclk_mhz",
        "uvd_vclk_mhz",
        "vce_eclk_mhz",
        "samu_clk_mhz",
    ])
    .ok();
    for r in roms {
        for (i, e) in r.powerplay.mm_table.iter().enumerate() {
            w.write_record([
                &r.file_name,
                &i.to_string(),
                &format!("{:.0}", e.uvd_dclk_mhz),
                &format!("{:.0}", e.uvd_vclk_mhz),
                &format!("{:.0}", e.vce_eclk_mhz),
                &format!("{:.0}", e.samu_clk_mhz),
            ])
            .ok();
        }
    }
}

fn vram_csv(w: &mut csv::Writer<Vec<u8>>, roms: &[&ParsedRom]) {
    w.write_record([
        "rom",
        "module",
        "part_number",
        "size_mb",
        "memory_type",
        "channels",
        "vendor_id_raw",
    ])
    .ok();
    for r in roms {
        for m in &r.vram.modules {
            w.write_record([
                &r.file_name,
                &m.index.to_string(),
                &m.part_number,
                &m.memory_size_mb.to_string(),
                &m.memory_type_name,
                &m.channel_num.to_string(),
                &m.vendor_id_raw.to_string(),
            ])
            .ok();
        }
    }
}

fn pcie_csv(w: &mut csv::Writer<Vec<u8>>, roms: &[&ParsedRom]) {
    w.write_record(["rom", "level", "pcie_gen", "pcie_lane_width"])
        .ok();
    for r in roms {
        for (i, e) in r.powerplay.pcie_table.iter().enumerate() {
            w.write_record([
                &r.file_name,
                &i.to_string(),
                &e.pcie_gen.to_string(),
                &e.pcie_lane_width.to_string(),
            ])
            .ok();
        }
    }
}

fn straps_csv(w: &mut csv::Writer<Vec<u8>>, roms: &[&ParsedRom]) {
    let max_regs = roms
        .iter()
        .flat_map(|r| r.vram.straps.iter())
        .map(|s| s.values.len())
        .max()
        .unwrap_or(0);
    let mut header = vec![
        "rom".to_string(),
        "memory_block".to_string(),
        "clock_mhz".to_string(),
        "effective_gbps".to_string(),
    ];
    for i in 0..max_regs {
        header.push(format!("reg{i}"));
    }
    w.write_record(header.iter().map(|s| s.as_str()).collect::<Vec<_>>())
        .ok();
    for r in roms {
        for s in &r.vram.straps {
            let mut cells: Vec<String> = vec![
                r.file_name.clone(),
                s.mem_block_id.to_string(),
                format!("{:.0}", s.clock_mhz),
                format!("{:.2}", s.effective_gbps),
            ];
            for i in 0..max_regs {
                cells.push(s.values.get(i).map(|v| v.to_string()).unwrap_or_default());
            }
            w.write_record(cells.iter().map(|s| s.as_str()).collect::<Vec<_>>())
                .ok();
        }
    }
}
