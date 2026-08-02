//! Simple ANSI colors, no external dependencies. Each function takes
//! text and returns it decorated, or plain text if `use_color` is
//! false - so all rendering logic is the same in both cases.

/// Aligns text to `width` columns (via `format!("{:<width$}")`) **without**
/// applying color. Golden rule used throughout the render module:
/// always align plain text first, and only then color it - never the
/// reverse. If already-colored text (with ANSI codes) is passed
/// to an outer `{:<width$}`, `format!` counts the invisible ANSI
/// bytes as visible characters and the padding comes out wrong,
/// misaligning the following column.
pub fn pad(s: &str, width: usize) -> String {
    format!("{s:<width$}")
}

/// Truncates (with "...") to fit in `max` visually. Used on
/// any value that may be longer than the column (file names,
/// concatenated lists, etc.) - without this, a long value
/// is simply not clipped by `{:<width$}` and overflows the column,
/// misaligning everything that follows in the same table.
pub fn truncate(s: &str, max: usize) -> String {
    let count = s.chars().count();
    if count <= max {
        s.to_string()
    } else if max <= 3 {
        "...".chars().take(max).collect()
    } else {
        let keep: String = s.chars().take(max - 3).collect();
        format!("{keep}...")
    }
}

/// Combines truncation + alignment - the pair used by default in every
/// table cell that may receive text longer than the column.
pub fn fit(s: &str, width: usize) -> String {
    pad(&truncate(s, width), width)
}

/// Truncates a string that may already contain ANSI codes, respecting a
/// limit of VISIBLE characters (escape codes don't count and are
/// never split in the middle). Used to fit already-colored content
/// into limited screen space (TUI panel) without breaking colors.
pub fn visible_truncate(s: &str, max_visible: usize) -> String {
    let mut out = String::new();
    let mut visible = 0usize;
    let mut truncated = false;
    let mut chars = s.chars().peekable();
    // Reserve room for the "...". Otherwise appending 3 chars after
    // filling max_visible would push the line past the panel edge.
    let limit = max_visible.saturating_sub(3);
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            out.push(c);
            if chars.peek() == Some(&'[') {
                out.push(chars.next().unwrap());
                for c2 in chars.by_ref() {
                    out.push(c2);
                    if c2.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        if visible >= limit {
            truncated = true;
            break;
        }
        out.push(c);
        visible += 1;
    }
    if truncated {
        out.push_str("...");
        out.push_str("\x1b[0m");
    }
    out
}

/// Wraps a possibly-ANSI-colored string into lines of at most
/// `max_visible` visible columns without ever splitting an escape
/// sequence in the middle. Continuation lines re-emit the SGR codes
/// that were active at the wrap point, so colors stay correct across
/// the break. Produces only the visible characters (no trailing reset
/// needed by the caller beyond what is embedded).
pub fn wrap_ansi(s: &str, max_visible: usize) -> Vec<String> {
    if max_visible == 0 {
        return Vec::new();
    }
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut cur_vis: usize = 0;
    let mut active: String = String::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        if c == '\u{1b}' {
            // Copy the whole escape sequence (SGR or otherwise) verbatim.
            let mut seq = String::from(c);
            i += 1;
            if i < chars.len() && chars[i] == '[' {
                seq.push('[');
                i += 1;
                while i < chars.len() {
                    let b = chars[i];
                    seq.push(b);
                    i += 1;
                    if b.is_ascii_alphabetic() {
                        break;
                    }
                }
                if seq.ends_with('m') {
                    // SGR: a reset clears the tracked codes, anything
                    // else adds to the re-emit prefix.
                    let params = seq.trim_end_matches('m').trim_start_matches("\x1b[");
                    if params.is_empty() || params == "0" {
                        active.clear();
                    } else {
                        active.push_str(&seq);
                    }
                }
            }
            cur.push_str(&seq);
            continue;
        }
        // The current line is full: the next visible char starts a fresh
        // line, carrying any active colors. (Each wrapped line is exactly
        // `max_visible` wide; only the last may be shorter.)
        if cur_vis >= max_visible {
            if !cur.ends_with("\x1b[0m") {
                cur.push_str("\x1b[0m");
            }
            lines.push(cur);
            cur = String::new();
            cur_vis = 0;
            cur.push_str(&active);
        }
        cur.push(c);
        cur_vis += 1;
        i += 1;
    }
    if !cur.is_empty() {
        if !cur.ends_with("\x1b[0m") {
            cur.push_str("\x1b[0m");
        }
        lines.push(cur);
    }
    lines
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub on: bool,
}

impl Palette {
    pub fn new(on: bool) -> Self {
        Palette { on }
    }

    fn wrap(&self, code: &str, s: &str) -> String {
        if self.on {
            format!("\x1b[{code}m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    }

    pub fn title(&self, s: &str) -> String {
        self.wrap("1;36", s) // cyan bold
    }
    pub fn label(&self, s: &str) -> String {
        self.wrap("2", s) // dim
    }
    pub fn value(&self, s: &str) -> String {
        self.wrap("1;33", s) // yellow bold
    }
    pub fn good(&self, s: &str) -> String {
        self.wrap("32", s) // green
    }
    pub fn warn(&self, s: &str) -> String {
        self.wrap("33", s) // yellow
    }
    pub fn bad(&self, s: &str) -> String {
        self.wrap("31", s) // red - used for validation warnings/errors
    }
    pub fn accent(&self, s: &str) -> String {
        self.wrap("1;35", s) // magenta bold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn visible(s: &str) -> usize {
        let mut in_esc = false;
        let mut n = 0;
        for c in s.chars() {
            if in_esc {
                if c.is_ascii_alphabetic() {
                    in_esc = false;
                }
                continue;
            }
            if c == '\u{1b}' {
                in_esc = true;
            } else {
                n += 1;
            }
        }
        n
    }

    #[test]
    fn wraps_plain_text_at_exact_width() {
        let lines = wrap_ansi("hello world", 5);
        assert_eq!(
            lines.iter().map(|l| visible(l)).collect::<Vec<_>>(),
            [5, 5, 1]
        );
    }

    #[test]
    fn keeps_colors_across_wrap() {
        let colored = format!("\x1b[32m{}\x1b[0m", "abcdefghij");
        let lines = wrap_ansi(&colored, 4);
        assert_eq!(lines.len(), 3);
        for l in &lines {
            assert!(
                l.starts_with("\x1b["),
                "continuation must re-emit color: {l:?}"
            );
            assert!(l.ends_with("\x1b[0m"), "every line must reset: {l:?}");
            assert!(visible(l) <= 4);
        }
    }

    #[test]
    fn single_line_under_limit_is_untouched() {
        let colored = format!("\x1b[1;33m{}\x1b[0m", "abc");
        let lines = wrap_ansi(&colored, 10);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], colored);
    }

    #[test]
    fn zero_width_yields_nothing() {
        assert!(wrap_ansi("abc", 0).is_empty());
    }

    #[test]
    fn short_lines_do_not_get_a_trailing_wrap() {
        let lines = wrap_ansi("hello", 10);
        assert_eq!(lines.len(), 1);
    }
}
