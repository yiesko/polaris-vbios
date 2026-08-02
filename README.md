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
- A tool cannot foresee every oddity; if in doubt, flash a verified
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

# Compare two ROMs side by side
polaris-vbios compare ~/GPU/XFX.RX480.4096.rom ~/GPU/XFX.RX580.8192.rom

# Interactive TUI
polaris-vbios tui ~/GPU/RX570_original.rom ~/GPU/Gigabyte.RX580.8192.rom
```

## Commands

| Command | Description |
|---------|-------------|
| `dump` | Read one or more ROMs - full detail, JSON or CSV |
| `compare` | Side-by-side comparison of two ROMs |
| `compare-all` | Matrix comparison of 3+ ROMs |
| `identify` | One-line summary per ROM |
| `patch` | Safe ROM editing (straps, PowerPlay, hex) + checksum fix |
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

---

## `identify` - summary mode

One line per ROM: manufacturer, ASIC family, VRAM capacity/type, boost
clocks, TDP and validation status:

```sh
polaris-vbios identify ~/GPU/RX570_original.rom

# Multiple ROMs at once
polaris-vbios identify ~/GPU/RX570_original.rom ~/GPU/Gigabyte.RX580.8192.rom

# All ROMs in a directory tree
polaris-vbios identify ~/GPU/**/*.rom
```

Example output:

```
RX570_original.rom            [Sapphire / PC Partner] Polaris/Tonga/Fiji  8192MB GDDR5 (Samsung)   boost 1284/1750MHz  TDP 145W  ✓
XFX.RX480.4096.160805.rom     [XFX]                   Polaris/Tonga/Fiji  4096MB GDDR5 (Samsung)   boost 1266/2000MHz  TDP 150W  ✓
Gigabyte.RX580.8192.rom       [Gigabyte]              Polaris/Tonga/Fiji  8192MB GDDR5 (Samsung)   boost 1360/2000MHz  TDP 150W  ✓
```

The ROM filename column is fixed at 32 characters: longer names are
truncated (with `...`) and shorter ones are padded, so every row's
`[vendor]` column stays aligned.

The `--no-color` flag disables ANSI output (useful for piping):

```sh
polaris-vbios identify ~/GPU/**/*.rom --no-color > rom_inventory.txt
```

---

## `dump` - full ROM read

Shows every parsed field for the selected sections:

```sh
# Everything (default: --sections all)
polaris-vbios dump ~/GPU/RX570_original.rom

# Specific sections only
polaris-vbios dump ~/GPU/RX570_original.rom --sections header,firmware

# Multiple ROMs - prints one after another
polaris-vbios dump ~/GPU/RX570_original.rom ~/GPU/XFX.RX580.8192.rom
```

### Sections

Available sections (use `polaris-vbios list-sections`):

| Key | Content |
|-----|---------|
| `header` | ATOM ROM header, Master Data Table, subsystem vendor/device ID, checksum, build date, present command tables |
| `pcir` | PCI Option ROM chain - x86 legacy + EFI images with vendor/device/class |
| `display` | Physical video outputs (DP, HDMI, DVI) and encoder chain (UniPHY) |
| `firmware` | Boot engine/memory clocks, reference clock, boot VDDC/VDDCI, PLL clock ranges, cooling solution, branding |
| `sclk` | SCLK P-States with resolved voltages and VDDC offsets, DPM state indices and classifications |
| `mclk` | MCLK P-States with VDDC/VDDCI/mVDD |
| `voltages` | VDDC/VDDGFX voltage lookup tables (LUT) |
| `vrm` | Voltage regulator configuration - GPIO, I2C (with chip ID), SVID2 |
| `mm` | Multimedia clocks - UVD DCLK/VCLK, VCE ECLK, SAMU CLK |
| `powertune` | PowerTune - TDP, TDC, TjMax, Hard Limits |
| `fan` | Fan curve - RPM max, PWM range, temperature targets |
| `pcie` | PCIe generation and lane width per level |
| `vram` | VRAM modules - part numbers, size, memory type, channel count, MC ucode version |
| `straps` | Memory strap registers by clock range |
| `caps` | PowerPlay platform capability flags |
| `asic` | ASIC layout - GFX IP version, shader engines, CUs, render backends (GFX_Info table) |
| `smu` | SMU firmware version, shared power source, SCLK FCW ranges (SMU_Info table) |
| `power` | Power sources - PCIe slot / 6-pin / 8-pin connectors, sensed power, sensor type |
| `gpio` | GPIO pin roles - pin index/bit shift and predefined IDs (PCIE_VDDC, VRHOT, PCC, AC/DC, ...) |
| `profiling` | ASIC profiling (ASIC_ProfilingInfo V3.6) - die min/max VDDC, load line slope, leakage/RO efuse mapping, EVV default VDDC, TDC limit per DPM, no-calc VDDC per DPM, AVFS enables |
| `ss` | Spread spectrum (ASIC_InternalSS_Info V3) - per-clock spread percentage, rate and centre/down+internal/external mode |

**Comma-separated:**

```sh
polaris-vbios dump ~/GPU/RX570_original.rom --sections vram,straps
polaris-vbios dump ~/GPU/**/*.rom --sections header,firmware,powertune
```

### Output formats

**Text** (default) - formatted tables with ANSI colors:

```sh
polaris-vbios dump ~/GPU/RX570_original.rom
```

**JSON** - all fields serialized:

```sh
polaris-vbios dump ~/GPU/RX570_original.rom --json
polaris-vbios dump ~/GPU/**/*.rom --json -o all_roms.json
```

With multiple ROMs, the output is a JSON array. Use `jq` for filtering:

```sh
polaris-vbios dump ~/GPU/**/*.rom --json | jq '.[].header.subsystem_vendor_name'
```

**CSV** - tabular export, requires exactly one tabular section:

```sh
polaris-vbios dump ~/GPU/**/*.rom --sections straps --format csv -o straps.csv
polaris-vbios dump ~/GPU/**/*.rom --sections vram --format csv
```

Exportable sections: `sclk`, `mclk`, `straps`, `mm`, `vram`, `pcie`.

The first column is always the ROM file name, so multiple ROMs can share
the same spreadsheet.

### Other dump options

| Flag | Description |
|------|-------------|
| `--no-color` | Disable ANSI colors |
| `-o`, `--output <file>` | Save to file instead of stdout |
| `--reg-names <file>` | Register annotation file (see below) |

```sh
polaris-vbios dump ~/GPU/RX570_original.rom --no-color -o output.txt
polaris-vbios dump ~/GPU/**/*.rom --json --no-color -o dump.json
```

---

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

---

## `patch` - safe ROM editing

Applies edits to a **copy** (never in place), recomputes the legacy
checksum, re-parses the result and sweeps every command table with the
disassembler - refusing to write if anything breaks. Every edit is
validated first:

- REFUSED: `--out` equal to the source (hard links included); input
  checksum invalid (unless `--fix-checksum`, which repairs the input
  before the edits are applied); offset out of file; `--hex` overlapping
  a protected layout area (boot area incl. the 0x48 entry point, the
  BIOS data area between it and the ATOM header - build date, vendor
  block, ATOM header, master table offsets, PCI data structure,
  sub-table headers); a strap/PP clock above the ROM's own Hard Limit
  table; clocks above 65535 MHz (they would silently wrap in the 100x
  table units); VDDC above the die maximum from ASIC_ProfilingInfo;
  `--set-strap`/`--set-strap-reg` targeting a register slot past the
  strap block's own size (would write into the next strap's clock
  field); `--retag-strap` above the highest strap clock the ROM ships
  (the memory controller only trains those straps); a no-op (value
  already identical).
- WARNED (applied anyway): unusual VDDC/TDP values; VDDC above the
  ROM's own VDDC hard limit (the SMC clamps); implausible SCLK/MCLK
  values (outside 100-2500 / 100-3000 MHz); `--hex` outside the
  checksum-covered region (may target the EFI/GOP image) or overlapping
  a parsed structure's data (PowerPlay/VRAM/straps/command table
  bytecode); overlapping edits (the later one wins); a retag with no
  matching MCLK DPM level.
- ALWAYS: `--dry-run` shows the edit plan (with old -> new values in
  human units) without writing; before writing, the result is verified
  structurally (no byte in the layout-defining areas may have changed
  outside the reported edits) and by a full re-parse plus disassembly
  sweep; the write itself is atomic (temp file + `fsync` + rename, so a
  power cut cannot leave a partial ROM).

```sh
# Set strap register 3 of the 2000 MHz strap (values in hex)
polaris-vbios patch rom.rom --out new.rom --set-strap 2000 3 0x12345678

# Same MC register (by absolute offset) in every strap block
polaris-vbios patch rom.rom --out new.rom --set-strap-reg 0x2B28 0x12345678

# Re-tag the 1625 MHz strap as 1650 MHz (keeps the block id)
polaris-vbios patch rom.rom --out new.rom --retag-strap 1625 1650

# PowerPlay: SCLK DPM level 2 to 1400 MHz + TDP to 135 W
polaris-vbios patch rom.rom --out new.rom --pp-sclk 2 1400 --pp-tdp 135

# VDDC LUT entry 5 to 950 mV; raw hex write (bytes separated by
# spaces and/or commas - "AA BB CC", "AA,BB,CC" or mixed are all fine)
polaris-vbios patch rom.rom --out new.rom --pp-vddc 5 950 --hex 0x1234 "00 FF"

# Repair the legacy checksum of a modified ROM
polaris-vbios patch rom.rom --out fixed.rom --fix-checksum

# Preview without writing anything
polaris-vbios patch rom.rom --out new.rom --pp-sclk 2 1400 --dry-run
```

Every edit is reported as `offset  old -> new  (description)` with
old/new values in human units, e.g.:

```
  0x009D0C  E0 73 01 00 -> E0 22 02 00  (SCLK DPM level 2: 1190 MHz -> 1400 MHz)
  0x009E80  78 00 -> 87 00  (PowerTune TDP: 120 W -> 135 W)
  0x00EBFF  FF -> 40  (legacy checksum byte)
```

The checksum fix adjusts the last byte of the declared region (0xFF
padding in all sampled ROMs); if that byte is not padding it scans back
up to 16 bytes and refuses otherwise.

---

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

---

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

---

## `tui` - interactive mode

Browse sections with arrow keys, toggle between ROMs, compare live:

```sh
# Single ROM
polaris-vbios tui ~/GPU/RX570_original.rom

# Two ROMs - press Tab to switch between A / B / Comparison views
polaris-vbios tui ~/GPU/RX570_original.rom ~/GPU/Gigabyte.RX580.8192.1737850666.rom
```

### Keybindings

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

---

## `--reg-names` - register annotation

Memory controller register names are **not publicly documented by AMD**.
Numeric indices are shown by default. If the community has mapped names
(e.g. via Polaris BIOS Editor), annotate them in a text file:

```
# Lines starting with # are ignored
0xA2F=MC_SEQ_CAS_TIMING
0xA30=MC_SEQ_CAS_TIMING2
2773=MC_SEQ_MISC1
```

Formats: `0xHEX=NAME`, `DECIMAL=NAME` or `HEX=NAME`.

```sh
polaris-vbios dump ~/GPU/RX570_original.rom --sections straps --reg-names ~/annotations.txt
polaris-vbios compare ~/GPU/RX570_original.rom ~/GPU/Gigabyte.RX580.8192.1737850666.rom \
  --sections straps --reg-names ~/annotations.txt
```

The same annotation file also decorates ATOM bytecode disassembly: pass
`--reg-names` to `disasm` or `diff-disasm` and register arguments in the
decoded instructions are shown as `REG[0x...] (name)` where a name is
defined:

```sh
polaris-vbios disasm ~/GPU/RX570_original.rom --reg-names ~/annotations.txt
polaris-vbios diff-disasm a.rom b.rom --reg-names ~/annotations.txt
```

Annotated names are explicitly marked as `"user annotation, not confirmed
by AMD"` in the output, never mixed with confirmed data.

---

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success - no validation warnings |
| 1 | Error - could not read or generate something |
| 2 | Success - but validation warnings found on at least one ROM |

---

## Validation

Every dump runs automatic sanity checks:

- Checksum verification
- PowerPlay table format revision vs expected family (Polaris = rev 7)
- SCLK/MCLK table: non-empty, boost clock in plausible range (800–2000 MHz)
- TDP: plausible range; typical Polaris is 75–185 W
- Temperature limits (TjMax): plausible range; Polaris maximum is ~105 °C
- Boot clocks present and non-zero
- FirmwareInfo structure present at expected offset
- Compute unit count (GFX_Info) vs the physical die derived from the
  device ID (e.g. 67DF = Polaris 10 with 36 CUs, 67EF = Polaris 11 with
  16 CUs, 67FF/699F = Polaris 12 with 16/10 CUs)
- VDDC usage vs the die voltage envelope from ASIC_ProfilingInfo:
  boot VDDC above the declared die maximum; VDDC LUT entries above it
  (clamped by the driver); profiling TDC vs PowerTune TDC inconsistency

Warnings are highlighted in the output header (or shown via `identify`).

The `straps` section (dump and compare) additionally shows an
**informational** hard-limit cross-check: strap clocks / SCLK DPM states
/ VDDC LUT entries above the ROM's own Hard Limit table. This is *not*
a validation warning - some stock ROMs (e.g. Lenovo/Medion RX560) ship
straps above their own limits; the driver/SMC simply clamps to the
limit. `patch` uses the same limits to refuse edits above them.

---

## Known limitations

- **SCLK voltage** is calculated by the SMU at runtime (AVFS), not read
  from the ROM as a direct value.
- **I2C VRM init pairs** - the official header calls them "voltage
  code/value (mV)", but real data often contains raw register writes
  (address + value). Displayed as raw pairs.
- **Build date** is heuristic (scanned from printable strings near the
  header), not a guaranteed official field.
- **MC register names** are not public; numeric indices only unless
  `--reg-names` is provided. Regulator, connector and manufacturer names
  come from official sources.
- **Hard Limit table** (PowerPlay) is present in only 2 of the 39
  sampled ROMs and its values sit below the cards' own boost clocks -
  treat it as a floor, not a guarantee of what the card can do.
- **Checksum fix** adjusts the last byte of the declared region (0xFF
  padding in every sampled ROM); ROMs without padding there are refused.
- **Boot risk of patches**: `patch` refuses clearly unsafe edits
  (over-limit clocks, broken structure) but no tool can guarantee a
  modified VBIOS boots - test on a secondary card with a flashable
  backup.

---

## What it reads

- **ATOM ROM header** - magic, size, Master Data Table pointers,
  subsystem vendor/device ID (with known vendor catalog)
- **PCI Option ROM (PCIR)** - x86 legacy and EFI (GOP) image detection
- **Video outputs** - `Object_Header` paths: physical connectors
  (DP/HDMI/DVI) and encoder chain (UniPHY)
- **FirmwareInfo** - boot engine/memory clocks, reference clock,
  PLL clock ranges, cooling solution, product branding
- **SCLK / MCLK P-States** - with resolved voltages, DPM state indices
  and PowerPlay classifications (BOOT, UI_PERFORMANCE, ...)
- **Voltage tables** - VDDC/VDDGFX LUT
- **VRM** - GPIO, I2C (regulator chip ID), SVID2 per rail
- **Multimedia** - UVD, VCE, SAMU clocks and VCE states
- **PowerTune** - TDP, TDC, Hard Limits, TjMax
- **ASIC layout** (`GFX_Info`) - GFX IP version, shader engines,
  compute units, render backends, tile pipes
- **SMU** (`SMU_Info`) - SMU firmware version, shared power source,
  SCLK FCW ranges (VCO + post-dividers)
- **Power sources** (`PowerSourceInfo`) - PCIe slot / 6-pin / 8-pin
  connectors, sensed power, GPIO/I2C sensors
- **GPIO pin roles** (`GPIO_Pin_LUT`) - pin index/bit shift mapped to
  predefined IDs (PCIE_VDDC, AC/DC, VRHOT, PCC, EFUSE, ...)
- **Fan curve** - RPM, PWM targets
- **PCIe** - generation and lane width per level
- **VRAM** - module count, part numbers, size, type, channels,
  MC ucode version
- **Memory straps** - MC register values grouped by clock range,
  per memory vendor when applicable
- **Platform caps** - PowerPlay capability flags
- **Command tables** - which of the 81 ATOM command tables
  (ASIC_Init, DIG1EncoderControl, GetVoltageInfo, ...) are present
- **VRAM ucode** (`MC_InitParameter`) - MC firmware version, ROM start
  address and length

---

## Parsing sources

Structures are parsed against the official AMD AtomBIOS definitions:
- `atombios.h` (AMD, MIT-licensed) - shipped in this repository for
  reference; section-specific structure sizes and revision gates follow
  it directly (e.g. `ATOM_GFX_INFO_V2_3`, `ATOM_SMU_INFO_V2_1`,
  `ATOM_POWER_SOURCE_INFO`, `ATOM_GPIO_PIN_LUT`)
- Linux kernel `drivers/gpu/drm/radeon/pptable.h` (GPL-2.0) - PowerPlay
  classification bits and UI-state decoding

---

## Code structure

```
src/
  lib.rs              library entry point: parses argv and dispatches (pub fn run)
  main.rs             thin binary wrapper: calls polaris_vbios::run()
  cli.rs              argument parsing (clap derive) + Command enum + --list-sections
  cmd/                per-command handlers, one file per subcommand
    mod.rs            dispatch table + shared exit-code logic (0/1/2) + write_output
    dump.rs           'dump'     handler (reads one or more ROMs)
    identify.rs       'identify' handler (summary mode)
    compare.rs        'compare' handler
    diff_disasm.rs    'diff-disasm' handler (diff logic lives in diff_disasm.rs)
    extract.rs        'extract' handler
    disasm.rs         'disasm' handler
    patch.rs          'patch' handler
  compare.rs          side-by-side comparison between two ROMs
  compare_all/        matrix comparison between 3+ ROMs
    mod.rs            driver
    header.rs, chip.rs, memory.rs, display.rs, power.rs   per-aspect cells
  compare_util.rs     shared comparison helpers (fmt_vals, note_push, finish_buf)
  diff_disasm.rs      offset-aligned unified diff of two ROMs' disassembly
  csv_export.rs       CSV export (csv crate)
  reg_names.rs        register annotation file loader
  tui.rs              interactive TUI (crossterm, no ratatui)
  rom/
    mod.rs            module root / shared re-exports
    reader.rs         little-endian byte reader with bounds checking
    header.rs         ATOM header + Master Data Table + subsystem ID + checksum + build date
    pci.rs            PCI Option ROM (PCIR) image chain
    display.rs        video outputs (Object_Header / SupportedDevicesInfo)
    firmware.rs       ATOM_FIRMWARE_INFO_V2_2
    powerplay.rs      ATOM_Tonga_POWERPLAYTABLE + all sub-tables
    vram.rs           VRAM_Info + memory straps
    vesa.rs           StandardVESA_Timing (native VESA modes)
    vrm.rs            ATOM_VOLTAGE_OBJECT_INFO_V3_1 (GPIO / I2C / SVID2 / EVV)
    asic.rs           ATOM_GFX_INFO_V2_1/V2_3 (ASIC layout)
    smu.rs            ATOM_SMU_INFO_V2_1 (SMU version + FCW ranges)
    gpio.rs           ATOM_GPIO_PIN_LUT (pin roles)
    gpio_i2c.rs       ATOM_GPIO_I2C_INFO (bus wiring)
    power_source.rs   ATOM_POWER_SOURCE_INFO (connectors/sensors)
    profiling.rs      ASIC_ProfilingInfo (die voltage range / efuse / TDC)
    ss.rs             ASIC_InternalSS_Info (spread spectrum)
    locate.rs         absolute offsets of editable fields (straps, PP sub-tables)
    patch/            patch engine: validated edits + checksum fix + verify
      mod.rs          public patch API
      apply.rs        writing edits into the ROM image
      checksum.rs     legacy checksum recompute + verify
      map.rs          ROM layout mapping (structure search/offsets)
      limits.rs       hard constraints for straps/PP edits
    disasm/           ATOM bytecode disassembler
      mod.rs          engine
      opcodes.rs      opcode table
    validate.rs       sanity checks + straps x hard-limit cross-check
    types.rs          data structs (serde-serializable, used by --json)
    types/            one module per domain (header, pci, chip, memory, power, ...)
  render/
    mod.rs            re-exports
    color.rs          ANSI colors + alignment/truncation helpers
    sections.rs       enum Section + selection via --sections
    text/             text rendering for each aspect
      header.rs, chip.rs, display.rs, memory.rs, power.rs
```

## License

Licensed under the **GNU General Public License v3.0** - see [LICENSE](LICENSE) for the full text.

## Credits

This project stands on the shoulders of public reference material and
community resources. Many thanks to:

- **AMD** for the AtomBIOS (ATI/AMD BIOS) format and its documentation -
  the `atombios.h` structure definitions (MIT-licensed, shipped in this
  repository) define every table this tool reads.
- The **Linux kernel community** (dri-devel / Radeon) - `pptable.h`,
  `atom.c`/`atom.h` and the Radeon/DCN driver sources, used as a
  reference for PowerPlay tables, command-table semantics and bytecode
  decoding.
- **TechPowerUp's VGA BIOS database** (techpowerup.com) - for hosting
  and making available the publicly-dumped Polaris VBIOS images used as
  the sample set (39 reference ROMs + 235 extended ROMs, RX 460 - RX 590)
  this project is verified against.
- **The VBIOS modding community** (TechPowerUp forums and related
  communities) - for the years of documented Polaris modding knowledge,
  clock/voltage surveys and checksum analysis that inform the `patch`
  and validator heuristics.

All trademarks belong to their
respective owners. AtomBIOS and Radeon are trademarks of AMD; this
project is neither affiliated with nor endorsed by AMD.

**Disclaimer**: ROM images are of others' property and remain so; this
tool only reads and (optionally) copies-modifies them on request, never
writes to a card.
