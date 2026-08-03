# Overview

Reader, dumper and comparator for **AMD Polaris** VBIOS (RX 4xx / RX 5xx -
Polaris10/Polaris11/Polaris20/Polaris21) in Rust. Reads the ROM directly
through the real AtomBIOS structures and displays everything in the terminal - plain text (CLI),
JSON, CSV or an interactive TUI.

The project was developed and verified against a collection of **274 real
Polaris BIOS images** (39 curated reference ROMs spanning every Polaris
family - RX 460 through RX 590 - plus an extended set of 235 ROMs from
the same sources). Every command, section and parser is exercised on that
whole set during development.

None of this means every ROM in the wild will parse perfectly: vendor
VBIOS images are inconsistent, some fields are heuristic, and edge cases
exist outside any sampled set. **Nothing reads or writes a GPU** - this
is a file tool only.

## Documentation

Per-topic docs live in `docs/`:

| File | Covers |
|------|--------|
| [docs/dump/README.md](docs/dump/README.md) | `dump` - sections, text/JSON/CSV output |
| [docs/compare/README.md](docs/compare/README.md) | `compare`, `compare-all`, `diff-disasm`, scriptable verdicts |
| [docs/patch/README.md](docs/patch/README.md) | `patch` - guards, `--timing`, `--force`, identity/VRAM editing |
| [docs/tui/README.md](docs/tui/README.md) | Interactive `tui` mode + keybindings |
| [docs/reference/README.md](docs/reference/README.md) | `check`/`convert`/`decode-strap`, `--reg-names`, validation, limitations |
| [docs/development/README.md](docs/development/README.md) | Parsing sources, code structure, license, credits |

## Patch warning

`patch` edits a ROM copy with validated, checksummed changes, but BIOS
modding is inherently risky. **Use at your own risk**: a badly applied
patch, an over-aggressive clock/voltage, or a wrong checksum can
brick a card, void its warranty and, at worst, corrupt the flash chip.
You are solely responsible for what you flash.

- Never `patch` in place - always write a copy with `--out` and keep a
  flash-backup of the stock ROM.
- Test modified images on a cheap secondary card first, never on the
  only card you own.
- A tool cannot foresee every oddity; if in doubt, flash the verified
  stock image and lose nothing.

## Build

Requires Rust 1.95+:

```sh
cargo build --release
# binary at target/release/polaris-vbios
```

Dependencies: `anyhow`, `serde`/`serde_json`, `crossterm`, `clap`, `csv`,
`base64`, `strip-ansi-escapes`.

## Quick start

```sh
# Identify all ROMs in a directory
polaris-vbios identify ~/GPU/*.rom

# Full dump of one ROM
polaris-vbios dump ~/GPU/xfx-rx570_original.rom

# Validate a ROM (scriptable exit codes)
polaris-vbios check ~/GPU/xfx-rx570_original.rom

# Compare two ROMs side by side
polaris-vbios compare ~/GPU/XFX.RX480.4096.rom ~/GPU/XFX.RX580.8192.rom

# Interactive TUI
polaris-vbios tui ~/GPU/RX570_original.rom ~/GPU/Gigabyte.RX580.8192.rom
```

## Commands

| Command | Description |
|---------|-------------|
| `dump` | Read one or more ROMs - full detail, JSON or CSV |
| `compare` | Side-by-side comparison of two ROMs (exit 1 when they differ) |
| `compare-all` | Matrix comparison of 3+ ROMs (exit 1 when they differ) |
| `identify` | One-line summary per ROM (or `--json`) |
| `patch` | Safe ROM editing (straps, timings, PowerPlay, hex) + checksum fix |
| `check` | Run every validation rule, scriptable exit codes (0/1/2) |
| `convert` | Convert a memory timing between cycles and nanoseconds |
| `decode-strap` | Decode a pasted memory strap register set (no ROM needed) |
| `diff-disasm` | Diff of the ATOM bytecode disassembly of two ROMs |
| `tui` | Interactive terminal UI |
| `list-sections` | List all available section keys |
| `completions` | Print a shell completion script (bash, elvish, fish, powershell, zsh) |
| `man` | Print a roff man page |
| `help` / `-h` | Show help for any command |

## Global options

| Flag | Description |
|------|-------------|
| `-h`, `--help` | Print help |
| `-V`, `--version` | Print version |
| `--list-sections` | List available section keys and exit |

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success - no validation warnings |
| 1 | Error (could not read/generate something), or `check` found findings, or `compare`/`compare-all` ROMs differ |
| 2 | Success but validation warnings found on at least one ROM (`check` uses it when a ROM fails to parse) |

- `check` - scripting contract: **0** clean, **1** findings reported,
  **2** could not parse a ROM.
- `compare`/`compare-all` - exit **1** exactly when the ROMs differ
  (text mode: the report's `≠` marker; JSON mode: structural
  comparison). If the ROMs are identical but still carry validation
  warnings, the exit is **2** like every other command.