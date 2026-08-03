# `compare`, `compare-all` and `diff-disasm`

## `compare` - two ROMs side by side

Shows numeric fields with **percentage delta**. Memory straps are shown
as a **register x clock matrix** (rows = MC registers, columns = strap
clocks, cells show A/B values; differing cells highlighted, rows without
differences hidden with `--diff-only`). Straps are matched by **clock
value**, not by position; both ROMs' straps also get the hard-limit
cross-check (informational).

```sh
polaris-vbios compare ~/GPU/RX570_original.rom ~/GPU/Gigabyte.RX580.8192.rom

# Only specific sections
polaris-vbios compare ~/GPU/RX570_original.rom ~/GPU/Gigabyte.RX580.8192.rom \
  --sections sclk,mclk,powertune,vram,straps

# Only fields that differ
polaris-vbios compare ~/GPU/RX570_original.rom ~/GPU/Gigabyte.RX580.8192.rom --diff-only

# Two XFX cards, different generations
polaris-vbios compare ~/GPU/XFX.RX480.4096.160805.rom ~/GPU/XFX.RX580.8192.rom \
  --sections header,display,straps --diff-only
```

Example output:

```
═══ Comparison: RX570_original.rom vs Gigabyte.RX580.8192.rom ═══

── SCLK P-States ──
  field                          A                    B
  ────────────────────────────── ────────────────────── ──────────────────────
  Levels (count)                 7                     7                     =
  level 0                        300.0 MHz             300.0 MHz              =
  level 1                        513.0 MHz             578.0 MHz              ≠
  level 2                        662.0 MHz             728.0 MHz              ≠
  level 3                        813.0 MHz             931.0 MHz              ≠
  level 4                        963.0 MHz             1080.0 MHz             ≠
  level 5                        1114.0 MHz            1228.0 MHz             ≠
  level 6                        1284.0 MHz            1360.0 MHz             ≠
```

With `--json`:

```sh
polaris-vbios compare ~/GPU/RX570_original.rom ~/GPU/Gigabyte.RX580.8192.1737850666.rom --json
```

`--reg-names` decorates the strap matrix the same way `dump` does.

## Scriptable verdict

Both `compare` and `compare-all` are scripting-friendly: the exit code
is **1 when the ROMs differ**, 0 when they are identical. In text mode
the report's own `≠` marker is the verdict; in JSON mode the two (or
more) documents are compared structurally.

```sh
polaris-vbios compare a.rom b.rom >/dev/null && echo identical || echo differ
```

## `compare-all` - matrix for 3+ ROMs

Same idea as `compare` but with one column per ROM:

```sh
# Three different cards
polaris-vbios compare-all \
  ~/GPU/RX570_original.rom \
  ~/GPU/XFX.RX480.4096.160805.rom \
  ~/GPU/Gigabyte.RX580.8192.1737850666.rom \
  --sections straps,powertune

# All ROMs in a directory, only differences
polaris-vbios compare-all ~/GPU/**/*.rom --diff-only

# Full comparison (all sections)
polaris-vbios compare-all ~/GPU/**/*.rom --json -o matrix.json
```

Straps are matched by clock across all ROMs simultaneously.

## `diff-disasm` - diff of the disassembly

Disassembles both ROMs' ATOM command tables and prints an offset-aligned
unified diff (addresses are relative to each table, so tables living at
different ROM offsets still line up). Default is all tables; `--table`
restricts to one (by index or name):

```sh
# All tables (default)
polaris-vbios diff-disasm a.rom b.rom

# Only table 10 (SetEngineClock)
polaris-vbios diff-disasm a.rom b.rom --table 10

# Only the lines that differ, across every table
polaris-vbios diff-disasm a.rom b.rom --diff-only

# Register annotation file applies to both ROMs
polaris-vbios diff-disasm a.rom b.rom --table 4 --reg-names mc-regs.txt
```

Tables present in only one ROM are flagged. Equal tables collapse to a
single `(identical)` line (hidden entirely with `--diff-only`).
