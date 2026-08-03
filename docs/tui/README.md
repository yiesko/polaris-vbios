# `tui` - interactive mode

Browse sections with arrow keys, toggle between ROMs, compare live:

```sh
# Single ROM
polaris-vbios tui ~/GPU/RX570_original.rom

# Two ROMs - press Tab to switch between A / B / Comparison views
polaris-vbios tui ~/GPU/RX570_original.rom ~/GPU/Gigabyte.RX580.8192.1737850666.rom
```

## Keybindings

| Key | Action |
|-----|--------|
| `↑`/`↓` or `j`/`k` | Navigate sections |
| `PgUp`/`PgDn` | Scroll (line indicator in footer) |
| `Tab` | Toggle view - ROM A / ROM B / Comparison (only with 2 ROMs) |
| `d` | Toggle diff-only mode in comparison view |
| `y` | Copy current section to system clipboard via OSC52 |
| `?` | Show help |
| `q` / `Esc` | Quit |

The clipboard (`y`) works in kitty, WezTerm, iTerm2, Windows Terminal,
Alacritty - no extra software needed.
