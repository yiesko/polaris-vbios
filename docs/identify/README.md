# `identify` - one-line summary per ROM

Prints a concise one-line summary for each ROM, showing the most
important fields: vendor, family, VRAM, boost clocks, and TDP.

```sh
# Single ROM
polaris-vbios identify ~/GPU/RX570_original.rom

# Multiple ROMs
polaris-vbios identify ~/GPU/*.rom

# JSON output (one object per ROM)
polaris-vbios identify ~/GPU/*.rom --json
```

## Output format

Text mode (default):

```
filename [vendor]   family               VRAM                        boost SCLK/MCLK  TDP  bootup_message  status
```

Example:

```
RX570_original.rom [XFX]   Polaris20 XL A1      8192MB GDDR5 (Samsung)    boost 1286/1750MHz  TDP 120W  D00033 Polaris20 XL A1  ✓
```

JSON mode (`--json`):

```json
[
  {
    "file": "RX570_original.rom",
    "vendor": "XFX",
    "family": "Polaris20 XL A1",
    "vram_size_mb": 8192,
    "memory_types": ["GDDR5"],
    "memory_vendors": ["Samsung"],
    "boost_sclk_mhz": 1286.0,
    "boost_mclk_mhz": 1750.0,
    "tdp_w": 120,
    "warnings": []
  }
]
```

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `roms` | Yes | One or more ROM files to identify |

## Flags

| Flag | Description |
|------|-------------|
| `--json` | Emit one JSON object per ROM instead of the one-line summary |
| `--no-color` | Disable colored output |

## Fields shown

- **vendor**: Subsystem vendor name (e.g. XFX, Sapphire, MSI)
- **family**: Detected GPU family (e.g. Polaris20 XL A1, Polaris10 XT)
- **vram_size_mb**: Total VRAM in megabytes
- **memory_types**: Memory type (GDDR5, GDDR5X, HBM2)
- **memory_vendors**: Memory vendor (Samsung, Hynix, Elpida/Micron)
- **boost_sclk_mhz**: Maximum engine clock in MHz
- **boost_mclk_mhz**: Maximum memory clock in MHz
- **tdp_w**: TDP in watts
- **status**: '✓' for clean ROMs, '⚠' N for ROMs with N warnings

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success - all ROMs parsed cleanly |
| 1 | Error (could not read/parse a ROM) |
| 2 | Success but validation warnings found on at least one ROM |
