# Utility commands: `list-sections`, `completions`, `man`, `help`

## `list-sections` - list available section keys

Prints the valid keys that can be used with `--sections` in `dump`
and `compare`:

```sh
polaris-vbios list-sections
```

Output:

```
Available sections (use with --sections, comma-separated):

  header     ATOM ROM header, Master Data Table, subsystem vendor/device ID, checksum, build date
  pcir       PCI Option ROM chain - x86 legacy + EFI images
  display    Video outputs (DP, HDMI, DVI) and encoder chain
  firmware   Boot clocks, reference clock, cooling solution, branding
  sclk       SCLK P-States with resolved voltages
  mclk       MCLK P-States
  voltages   VDDC/VDDGFX voltage lookup tables
  vrm        Voltage regulator configuration
  mm         Multimedia clocks (UVD/VCE/SAMU)
  powertune  PowerTune - TDP, TDC, TjMax
  fan        Fan curve - RPM max, PWM range, temperature targets
  pcie       PCIe generation and lane width
  vram       VRAM modules - part numbers, size, memory type
  straps     Memory strap registers by clock range
  caps       PowerPlay platform capability flags
  asic       ASIC layout (GFX_Info table)
  smu        SMU firmware version, FCW ranges
  power      Power sources (PCIe slot / 6-pin / 8-pin)
  gpio       GPIO pin roles
  profiling  ASIC profiling (die voltage range, leakage)
  ss         Spread spectrum
  all        All sections above (default)
```

Also available as a global flag: `polaris-vbios --list-sections`

## `completions` - shell completion scripts

Print a shell completion script to stdout. Use `>`
redirection to save it:

```sh
# Bash
polaris-vbios completions bash > ~/.bashrc.d/polaris-vbios-completions.bash

# Zsh
polaris-vbios completions zsh > ~/.zfunc/_polaris-vbios

# Fish
polaris-vbios completions fish > ~/.config/fish/completions/polaris-vbios.fish

# Elvish
polaris-vbios completions elvish > ~/.config/elvish/completions/polaris-vbios.elv

# Powershell
polaris-vbios completions powershell > polaris-vbios-completions.psm1
```

Supported shells: `bash`, `elvish`, `fish`, `powershell`, `zsh`

## `man` - generate man page

Print a roff man page to stdout. Use `>` redirection or pipe to
`man` directly:

```sh
# Save to file
polaris-vbios man > polaris-vbios.1

# View directly (requires a man-capable terminal)
polaris-vbios man | man -l -
```

## `help` / `-h` - help

Show help for any command:

```sh
# General help
polaris-vbios --help

# Command-specific help
polaris-vbios dump --help
polaris-vbios patch --help
polaris-vbios transplant --help
```

## Exit codes

All utility commands exit with code **0** on success.
