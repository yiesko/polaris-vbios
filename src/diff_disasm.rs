use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};

use crate::render::color::Palette;
use crate::rom::disasm::{DisasmLine, TableDisasm, disasm_command_tables};
use crate::rom::reader::Reader;

/// Offset-aligned diff of the disassembly of two ROMs' command tables.
/// The disasm `addr` field is relative to the table start, so tables at
/// different ROM offsets still line up.
pub fn run(
    a_path: &Path,
    b_path: &Path,
    table_filter: Option<&str>,
    diff_only: bool,
    color: bool,
    reg_names: Option<&HashMap<u16, String>>,
) -> Result<String> {
    let (a, b) = decode_pair(a_path, b_path, table_filter, reg_names)?;
    let pal = Palette::new(color);
    let mut out = String::new();
    let indexes = {
        let mut v: Vec<usize> = a.keys().chain(b.keys()).copied().collect();
        v.sort_unstable();
        v.dedup();
        v
    };

    for idx in indexes {
        match (a.get(&idx), b.get(&idx)) {
            (Some(ta), Some(tb)) => {
                out.push_str(&pal.title(&format!(
                    "\n── {index:02} {name} (rev {fmt}.{cont})",
                    index = ta.index,
                    name = ta.name,
                    fmt = ta.fmt_rev,
                    cont = ta.cont_rev,
                )));
                out.push('\n');
                let d = diff_table(ta, tb, &pal, diff_only);
                out.push_str(&d);
            }
            (Some(ta), None) => {
                out.push_str(&pal.warn(&format!(
                    "\n── {index:02} {name}: present only in A",
                    index = ta.index,
                    name = ta.name,
                )));
                out.push('\n');
                if !diff_only {
                    for l in &ta.lines {
                        out.push_str(&format!(
                            "  {addr:04X}  {text}\n",
                            addr = l.addr,
                            text = l.text
                        ));
                    }
                }
            }
            (None, Some(tb)) => {
                out.push_str(&pal.warn(&format!(
                    "\n── {index:02} {name}: present only in B",
                    index = tb.index,
                    name = tb.name,
                )));
                out.push('\n');
                if !diff_only {
                    for l in &tb.lines {
                        out.push_str(&format!(
                            "  {addr:04X}  {text}\n",
                            addr = l.addr,
                            text = l.text
                        ));
                    }
                }
            }
            (None, None) => unreachable!(),
        }
    }
    Ok(out)
}

fn decode_pair(
    a_path: &Path,
    b_path: &Path,
    table_filter: Option<&str>,
    reg_names: Option<&HashMap<u16, String>>,
) -> Result<(HashMap<usize, TableDisasm>, HashMap<usize, TableDisasm>)> {
    let decode = |path: &Path| -> Result<Vec<TableDisasm>> {
        let data =
            std::fs::read(path).with_context(|| format!("could not read '{}'", path.display()))?;
        let r = Reader::new(&data);
        let header = crate::rom::header::parse_rom_header(&r)
            .with_context(|| format!("invalid ATOM header in '{}'", path.display()))?;
        disasm_command_tables(&r, header.master_cmd_table_offset, table_filter, reg_names)
            .with_context(|| format!("disasm failed for '{}'", path.display()))
    };
    let a = decode(a_path)?;
    let b = decode(b_path)?;
    let into_map = |tables: Vec<TableDisasm>| -> HashMap<usize, TableDisasm> {
        tables.into_iter().map(|t| (t.index, t)).collect()
    };
    Ok((into_map(a), into_map(b)))
}

/// Merge-diffs two tables' line lists (both sorted by `addr`), with a
/// rolling context of 2 unchanged lines. `-` lines come from A, `+`
/// lines from B; matching lines are context.
fn diff_table(ta: &TableDisasm, tb: &TableDisasm, pal: &Palette, diff_only: bool) -> String {
    const CONTEXT: usize = 2;
    let mut out = String::new();
    let mut ctx: Vec<&DisasmLine> = Vec::new();
    let mut a_changed = 0usize;
    let mut b_changed = 0usize;

    let (mut i, mut j) = (0usize, 0usize);
    while i < ta.lines.len() || j < tb.lines.len() {
        match (ta.lines.get(i), tb.lines.get(j)) {
            (Some(x), Some(y)) if x.addr == y.addr => {
                if x.text == y.text {
                    if !diff_only {
                        ctx.push(x);
                        if ctx.len() > CONTEXT {
                            ctx.remove(0);
                        }
                    }
                    i += 1;
                    j += 1;
                } else {
                    for l in ctx.drain(..) {
                        out.push_str(&format!(
                            "  {addr:04X}  {text}\n",
                            addr = l.addr,
                            text = l.text
                        ));
                    }
                    out.push_str(&format!(
                        "{} {addr:04X}  {text}\n",
                        pal.bad("-"),
                        addr = x.addr,
                        text = x.text
                    ));
                    out.push_str(&format!(
                        "{} {addr:04X}  {text}\n",
                        pal.good("+"),
                        addr = y.addr,
                        text = y.text
                    ));
                    a_changed += 1;
                    b_changed += 1;
                    i += 1;
                    j += 1;
                }
            }
            (Some(x), Some(y)) if x.addr < y.addr => {
                for l in ctx.drain(..) {
                    out.push_str(&format!(
                        "  {addr:04X}  {text}\n",
                        addr = l.addr,
                        text = l.text
                    ));
                }
                out.push_str(&format!(
                    "{} {addr:04X}  {text}\n",
                    pal.bad("-"),
                    addr = x.addr,
                    text = x.text
                ));
                a_changed += 1;
                i += 1;
            }
            (Some(x), None) => {
                for l in ctx.drain(..) {
                    out.push_str(&format!(
                        "  {addr:04X}  {text}\n",
                        addr = l.addr,
                        text = l.text
                    ));
                }
                out.push_str(&format!(
                    "{} {addr:04X}  {text}\n",
                    pal.bad("-"),
                    addr = x.addr,
                    text = x.text
                ));
                a_changed += 1;
                i += 1;
            }
            (Some(_), Some(y)) | (None, Some(y)) => {
                for l in ctx.drain(..) {
                    out.push_str(&format!(
                        "  {addr:04X}  {text}\n",
                        addr = l.addr,
                        text = l.text
                    ));
                }
                out.push_str(&format!(
                    "{} {addr:04X}  {text}\n",
                    pal.good("+"),
                    addr = y.addr,
                    text = y.text
                ));
                b_changed += 1;
                j += 1;
            }
            (None, None) => unreachable!(),
        }
    }

    if diff_only && a_changed == 0 && b_changed == 0 {
        return String::new();
    }
    let summary = if a_changed == 0 && b_changed == 0 {
        "  (identical)\n".to_string()
    } else {
        format!(
            "  {} {} changed line(s) in A, {} in B ({} total)\n",
            pal.warn("≠"),
            a_changed,
            b_changed,
            a_changed + b_changed
        )
    };
    format!("{summary}{out}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sapphire() -> std::path::PathBuf {
        std::path::PathBuf::from("samples/BIOS/RX570/Sapphire.RX570.4096.170317_2.rom")
    }
    fn asus() -> std::path::PathBuf {
        std::path::PathBuf::from("samples/BIOS/RX570/Asus.RX570.4096.170508.rom")
    }

    /// Spec test pair: Sapphire vs Asus RX570. Table 4 (DIGxEncoderControl)
    /// and table 10 (SetEngineClock) are byte-identical across the pair;
    /// the overall diff (all tables) has differences.
    #[test]
    fn known_pair() {
        if !sapphire().exists() || !asus().exists() {
            eprintln!("sample ROMs not present; skipping diff-disasm pair test");
            return;
        }
        let t4 = run(&sapphire(), &asus(), Some("4"), false, false, None).unwrap();
        assert!(t4.contains("04 DIGxEncoderControl"), "table 4 header: {t4}");
        assert!(t4.contains("(identical)"), "table 4 identical: {t4}");

        let t10 = run(&sapphire(), &asus(), Some("10"), false, false, None).unwrap();
        assert!(t10.contains("10 SetEngineClock"), "table 10 header: {t10}");
        assert!(t10.contains("(identical)"), "table 10 identical: {t10}");

        let all = run(&sapphire(), &asus(), None, true, false, None).unwrap();
        assert!(all.contains("≠"), "all-tables diff has differences: {all}");
        assert!(all.contains("──"), "all-tables diff has table headers");
    }
}
