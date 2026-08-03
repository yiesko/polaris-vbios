# Reference: `--reg-names`, validation, limitations, internals

## Scripting: `check`, `convert`, `decode-strap`

### `check` - run every validation rule

Validates one or more ROMs against all the checks in the
[Validation](#validation) section. Scriptable: **0** when every ROM
parses cleanly, **1** when findings are reported, **2** when a ROM
fails to parse. `--quiet` prints nothing - only the exit code:

```sh
polaris-vbios check ~/GPU/RX570_original.rom
polaris-vbios check ~/GPU/**/*.rom --quiet; echo "exit: $?"
```

### `convert` - cycles <-> nanoseconds

Convert a memory timing between clock cycles and nanoseconds at a given
memory clock. Give exactly one of `--cycles` or `--ns`:

```sh
polaris-vbios convert --clock 2000 --cycles 219   # 219 cycles = 109.5 ns at 2000 MHz
polaris-vbios convert --clock 2000 --ns 110       # 110 ns = 220 cycles at 2000 MHz
```

Useful for planning `patch --timing` values.

### `decode-strap` - decode pasted strap hex

Decode a memory strap register set **without a ROM**, for working with
values pasted from a tool or a Hex dump. Uses the default 14-slot
Polaris register index table unless `--indices` overrides it:

```sh
# One hex u32 per strap register slot (same sizing as the middle of
# the memory strap blob), clock for the cycles->ns conversion
polaris-vbios decode-strap 2000 0x001CAA22 0x0000000F 0x001B0600 ...

# Override the register index table
polaris-vbios decode-strap 2000 0xA2F 0x11 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 \
  --indices 0xA2F,0xA30,0xAD5,0xA2C,0xA28,0xA29,0xA2A,0xA2B,0xA81,0xA8B,0xA5F,0x9DD,0x9DE,0xFFFF
```

The core timing fields (`tCL`, `tRFC`, `tRC`, `tRP`, `tRRD`, `tFAW`, ...)
are decoded into cycles - nanoseconds are appended for the fields that
are stored as time (`tRC`/`tRFC`/`tRP`/`tRRD`/`tFAW`). Every slot is
then listed per register with its decoded fields and raw hex. This
mirrors what `dump --sections straps` shows for a real ROM - the same
`CORE_TIMINGS`/register table.

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

## Validation

Every dump runs automatic sanity checks:

- Checksum verification
- PowerPlay table format revision vs expected family (Polaris = rev 7)
- SCLK/MCLK table: non-empty, boost clock in plausible range (800–2000 MHz)
- TDP: judged against the **die family's envelope** (Baffin 42–75 W,
  Lexa 35–65 W, Ellesmere 10 85–130 W, Ellesmere 20 60–185 W, Polaris 30
  185–220 W) plus the real-world OC headroom of that die - ranges are
  measured from stock factory VBIOS, so a reference RX 470 (85 W), RX
  480 (110 W), RX 550 (35 W) or RX 590 (185 W) never trips
- Die detection for the TDP envelope uses, in order: the BIOS bootup
  message (whitespace-insensitive: "POLARIS 30 XT A1" is matched), the
  MC microcode version (12 nm Polaris 30 ships 11853696+; 14 nm dies
  stay ~11850240–11852848), and for 67DF boards whose boot string does
  not name the die (Asus "67DFHB...", MSI "113-MSI...", Gigabyte
  "GV-...", Sapphire "E347/E353...") the union of Polaris 10 + Polaris
  20 (RX 470–580)
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
The `check` command runs the same rules with a scriptable exit code
(0 clean / 1 findings / 2 could not parse), see the main README's
exit-code table.

The `straps` section (dump and compare) additionally shows an
**informational** hard-limit cross-check: strap clocks / SCLK DPM states
/ VDDC LUT entries above the ROM's own Hard Limit table. This is *not*
a validation warning - some stock ROMs (e.g. Lenovo/Medion RX560) ship
straps above their own limits; the driver/SMC simply clamps to the
limit. `patch` uses the same limits to refuse edits above them.

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
