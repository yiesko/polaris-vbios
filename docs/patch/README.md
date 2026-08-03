# `patch` - safe ROM editing

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

## Named timing edits (`--timing`)

Edit a named timing field by name, in cycles or nanoseconds, without
knowing any offset or bitfield. The value is applied to **every** strap
block of that clock (all memory blocks share one strap list). A value
carrying the `ns` suffix is converted to cycles at the target clock:

```sh
# tRFC of the 2000 MHz strap: 219 cycles (= 109.5 ns at 2000 MHz)
polaris-vbios patch rom.rom --out new.rom --timing 2000 tRFC 219

# Same thing, written directly in nanoseconds
polaris-vbios patch rom.rom --out new.rom --timing 2000 tRFC 109.5ns

# Several fields on one clock in a single call
polaris-vbios patch rom.rom --out new.rom --timing 2000 tRFC 219 --timing 2000 tCL 22
```

Recognized fields: `tCL`, `tRCDW`, `tRCDWA`, `tRCDR`, `tRCDRA`, `tRRD`,
`tRC`, `tRP`, `tRFC`, `tFAW`. Values that do not fit the field's bit
width are refused before anything is written (e.g. `tRFC` is a 9-bit
field, so 512 cycles does not fit and is rejected). Use
`convert` to plan the cycles/ns numbers and `decode-strap` to read a
pasted strap set.

## Power guardrails and `--force`

TDP edits are judged against the **die family's envelope**, not one
global range: an RX 460 at 150 W is suspicious, an RX 580 "premium" at
185 W is fine. The same envelopes drive the `check` findings and the
`dump`/`identify` warnings.

- **Refused** (unless `--force`): a TDP that makes no physical sense
  for the die - below the factory floor (e.g. 5 W) or above the
  real-world OC ceiling plus 25% headroom (e.g. 300 W on a 120 W
  single 6-pin RX 570) - and a TDP above a **real** max-power-delivery
  ceiling the ROM declares.
- **Warned** (applied anyway): TDP above the factory range but within
  what real-world OC reports reach; TDP above the ROM's configurable
  limit (the SMC clamps to the safe value); TDP above a
  max-power-delivery that is stock filler rather than a real headroom
  cap (125 W on a 120 W RX 570 is filler - a legitimate 130 W OC must
  not be blocked).
- `--force` lifts **only** those power guardrails - checksum, clock
  hard limits and truncation still refuse.
- `--dry-run` never refuses: it prints the plan's guardrail warnings so
  you can decide whether `--force` is needed.

```sh
# Absurd for a Polaris 20: refused, nothing written
polaris-vbios patch rom.rom --out out.rom --pp-tdp 300

# The same value, dry-run: plan + warning shown, nothing written
polaris-vbios patch rom.rom --out out.rom --pp-tdp 300 --dry-run

# Decision made: --force writes it anyway, still warning loudly
polaris-vbios patch rom.rom --out out.rom --pp-tdp 300 --force

# Legit OC within the envelope: applied with an informational warning
polaris-vbios patch rom.rom --out out.rom --pp-tdp 200
```

## Identity and VRAM editing

- `--clone-ids <REF.ROM>` copies the device identity from a reference
  ROM: the device-id into every PCI option ROM image (legacy + EFI PCIR)
  and the subsystem vendor/device pair into the ATOM header. Warns
  (non-blocking) when the two ROMs' device-ids map to different dies
  (e.g. cloning a Polaris 10 id onto a Polaris 12 ROM).
- `--import-vram <REF.ROM>` replaces the **entire** VRAM_Info table -
  VRAM modules, memory straps and the MC tuning sub-tables - with the
  reference ROM's factory-calibrated block. This is the safe way to
  change memory size: it brings a coherent, calibrated set (e.g. a 4 GB
  RX 570 ROM whose board actually holds 8 GB Hynix H5GC8H24AJR can be
  made into a proper 8 GB ROM by importing the matching official 8 GB
  BIOS). REFUSED when the reference is internally incoherent (strap
  blocks targeting empty modules, mixed densities), when its format
  differs, or when its table does not fit the destination's layout.
- `--vram-size-mb <N>` writes geometry only: `usMemorySize`/`ucDensity`
  on every VRAM module, **without touching the straps** (geometry
  changes, timing doesn't). REFUSED unless a strap block is calibrated
  for the requested size; with `--i-understand-strap-mismatch` the
  edit is forced after printing the mismatch (modules, straps, and a
  concrete value divergence at a shared clock). `--import-vram` is the
  proper path for size changes; `--vram-size-mb` is the escape hatch.
  Mutually exclusive with `--import-vram`; `--clone-ids` composes with
  both.

```sh
# Make the 4 GB XFX RX 570 ROM (8 GB physical VRAM) an 8 GB ROM
polaris-vbios patch XFX.RX570.4096.170419.rom --out xfx8gb.rom \
  --import-vram 247426.rom --dry-run            # inspect first
polaris-vbios patch XFX.RX570.4096.170419.rom --out xfx8gb.rom \
  --import-vram 247426.rom                      # write

# Same board, different brand: clone identity + import VRAM in one go
polaris-vbios patch other-brand.rom --out fixed.rom \
  --clone-ids 247426.rom --import-vram 247426.rom

# Declared geometry only (straps stay calibrated for the old density)
polaris-vbios patch rom.rom --out geom.rom \
  --vram-size-mb 8192 --i-understand-strap-mismatch
```

Every edit is reported as `offset  old -> new  (description)` with
old/new values in human units (numeric edits carry a percentage delta),
e.g.:

```
  0x009D0C  E0 73 01 00 -> E0 22 02 00  (SCLK DPM level 2: 1190 MHz -> 1400 MHz)
  0x009E80  78 00 -> 87 00  (PowerTune TDP: 120 W -> 135 W (+12.5%))
  0x00EBFF  FF -> 40  (legacy checksum byte)
```

The checksum fix adjusts the last byte of the declared region (0xFF
padding in all sampled ROMs); if that byte is not padding it scans back
up to 16 bytes and refuses otherwise.