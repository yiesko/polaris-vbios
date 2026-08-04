# `disasm` - ATOM command table disassembly

Disassemble the ATOM bytecode of the command tables in a ROM. Each
command table contains hand-written ATOM interpreter bytecode that
the GPU's system management unit (SMU) executes for hardware
initialization.

```sh
# Disassemble all command tables
polaris-vbios disasm ~/GPU/RX570_original.rom

# Disassemble a specific table by name
polaris-vbios disasm ~/GPU/RX570_original.rom --table SetEngineClock

# Disassemble a specific table by index
polaris-vbios disasm ~/GPU/RX570_original.rom --table 12

# With register annotations
polaris-vbios disasm ~/GPU/RX570_original.rom --reg-names ~/annotations.txt
```

## Output format

Each table is printed as a header followed by offset+opcode lines:

```
── 12 SetEngineClock (rev 1.2, ws 4, ps 2, 128 bytes) @ 0x9764
  0000  PS_WS12_GETASICINITDATAENTRY, ps_arg[0]
  0002  PS_WS12_EOT
  0004  PS_WS12_GETENGINECLKFREQ, ps_arg[0], ws_arg[1]
  ...
```

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `rom` | Yes | ROM file to disassemble |

## Flags

| Flag | Description |
|------|-------------|
| `--table` | Only disassemble this table (by name or index) |
| `--reg-names` | Register annotation file for decoded register arguments |
| `--no-color` | Disable colored output |

## Related commands

- `diff-disasm` - diff the disassembly of two ROMs' command tables
- `dump` - display parsed command table metadata (names, sizes, offsets)
