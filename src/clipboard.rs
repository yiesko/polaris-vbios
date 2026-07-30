use std::io::{self, Write};

use base64::Engine;

/// Copies text to the system clipboard via OSC52 — works in modern
/// terminals (kitty, WezTerm, iTerm2, Windows Terminal,
/// default-config Alacritty) without needing any clipboard library
/// or X11/Wayland/win32 access: it is just an escape sequence that
/// the terminal itself intercepts.
pub fn copy_to_clipboard(text: &str) -> io::Result<()> {
    let plain = strip_ansi_escapes::strip_str(text);
    let b64 = base64::engine::general_purpose::STANDARD.encode(plain.as_bytes());
    let mut out = io::stdout();
    write!(out, "\x1b]52;c;{b64}\x07")?;
    out.flush()
}
