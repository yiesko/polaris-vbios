use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Loads a register annotation file in the format:
///   0xA2F=MC_SEQ_CAS_TIMING
///   2608=MC_SEQ_CAS_TIMING2
///
/// Accepts hex (with 0x prefix) or decimal indices. Blank lines and
/// lines starting with '#' are ignored. This program never ships with
/// pre-loaded names - it is entirely what the user provides here,
/// so any name shown in the straps section when this file is used is
/// explicitly marked as a user annotation, not a fact confirmed by AMD.
pub fn load(path: &Path) -> Result<HashMap<u16, String>, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("could not read '{}': {e}", path.display()))?;
    let mut map = HashMap::new();
    for (lineno, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, name)) = line.split_once('=') else {
            return Err(format!(
                "line {} of '{}' is invalid (expected 'INDEX=NAME'): {line:?}",
                lineno + 1,
                path.display()
            ));
        };
        let key = key.trim();
        let name = name.trim();
        let index: u16 =
            if let Some(hex) = key.strip_prefix("0x").or_else(|| key.strip_prefix("0X")) {
                u16::from_str_radix(hex, 16).map_err(|_| {
                    format!(
                        "line {} of '{}': invalid hex index: {key:?}",
                        lineno + 1,
                        path.display()
                    )
                })?
            } else {
                key.parse().map_err(|_| {
                    format!(
                        "line {} of '{}': invalid index (use decimal or 0xHEX): {key:?}",
                        lineno + 1,
                        path.display()
                    )
                })?
            };
        if name.is_empty() {
            return Err(format!(
                "line {} of '{}': empty name for index {key}",
                lineno + 1,
                path.display()
            ));
        }
        map.insert(index, name.to_string());
    }
    Ok(map)
}
