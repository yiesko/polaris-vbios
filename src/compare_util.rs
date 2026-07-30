/// Joins values with spaces - the repeated
/// `.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" ")`
/// pattern extracted to a single helper.
pub fn fmt_vals(values: &[u32]) -> String {
    values
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn note_push(buf: &mut String, any_row: &mut bool, s: &str) {
    *any_row = true;
    buf.push_str(s);
    buf.push('\n');
}

/// Collects non-empty part numbers from VRAM modules.
pub fn part_numbers(modules: &[crate::rom::types::VramModule]) -> Vec<String> {
    modules
        .iter()
        .filter(|m| !m.part_number.is_empty())
        .map(|m| m.part_number.clone())
        .collect()
}

pub fn finish_buf(mut buf: String, any_row: bool, empty_message: &str) -> String {
    if !any_row {
        buf.push_str(&format!("  {empty_message}\n"));
    }
    buf
}
