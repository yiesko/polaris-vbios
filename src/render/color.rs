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

/// Truncates (with "...") to fit in `max` visible characters. Used on
/// any value that may be longer than the column (file names,
/// concatenated lists, etc.) - without this, a long value
/// is simply not clipped by `{:<width$}` and overflows the column,
/// misaligning everything that follows in the same table.
pub fn truncate(s: &str, max: usize) -> String {
    let count = s.chars().count();
    if count <= max {
        s.to_string()
    } else if max == 0 {
        String::new()
    } else {
        let keep: String = s.chars().take(max - 1).collect();
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
        if visible >= max_visible {
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
