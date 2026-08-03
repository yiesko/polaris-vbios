# `dump` - full ROM read

Shows every parsed field for the selected sections:

```sh
# Everything (default: --sections all)
polaris-vbios dump ~/GPU/RX570_original.rom

# Specific sections only
polaris-vbios dump ~/GPU/RX570_original.rom --sections header,firmware

# Multiple ROMs - prints one after another
polaris-vbios dump ~/GPU/RX570_original.rom ~/GPU/XFX.RX580.8192.rom
```

## Sections

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

## Output formats

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

## Other dump options

| Flag | Description |
|------|-------------|
| `--no-color` | Disable ANSI colors |
| `-o`, `--output <file>` | Save to file instead of stdout |
| `--reg-names <file>` | Register annotation file (see the reference docs) |

```sh
polaris-vbios dump ~/GPU/RX570_original.rom --no-color -o output.txt
polaris-vbios dump ~/GPU/**/*.rom --json --no-color -o dump.json
```
