# Development: parsing sources, code structure, license, credits

## Parsing sources

Structures are parsed against the official AMD AtomBIOS definitions:
- `atombios.h` (AMD, MIT-licensed) - shipped in this repository for
  reference; section-specific structure sizes and revision gates follow
  it directly (e.g. `ATOM_GFX_INFO_V2_3`, `ATOM_SMU_INFO_V2_1`,
  `ATOM_POWER_SOURCE_INFO`, `ATOM_GPIO_PIN_LUT`)
- Linux kernel `drivers/gpu/drm/radeon/pptable.h` (GPL-2.0) - PowerPlay
  classification bits and UI-state decoding

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
    transplant.rs     'transplant' handler (PCI ROM image swapping)
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

Licensed under the **GNU General Public License v3.0** - see [LICENSE](../../LICENSE) for the full text.

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
