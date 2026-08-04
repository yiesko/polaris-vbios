# `extract` - dump PCI ROM images to files

Extract the firmware images (Legacy x86 / EFI GOP) from the PCI
option ROM chain to individual binary files, or print their metadata
as JSON.

```sh
# Extract all images (legacy + EFI)
polaris-vbios extract ~/GPU/RX570_original.rom

# Extract only EFI image
polaris-vbios extract ~/GPU/RX570_original.rom --image efi

# Extract only Legacy image
polaris-vbios extract ~/GPU/RX570_original.rom --image legacy

# Extract to a specific directory
polaris-vbios extract ~/GPU/RX570_original.rom --output /tmp/images/

# Print metadata as JSON (no files written)
polaris-vbios extract ~/GPU/RX570_original.rom --json
```

## Output files

When not using `--json`, the command writes one file per image:

```
{rom_stem}.{index}-{type}.bin
```

Example for `RX570_original.rom`:

```
RX570_original.0-legacy.bin
RX570_original.1-efi.bin
```

Each file contains the raw bytes of that PCI ROM image, from the
image's start offset to its declared size (or end of file, whichever
is smaller).

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `rom` | Yes | ROM file to extract images from |

## Flags

| Flag | Description |
|------|-------------|
| `--image` | Which image to extract: `efi`, `legacy` or `all` (default: `all`) |
| `--output`, `-o` | Output directory (created if missing, default: current dir) |
| `--json` | Print image metadata as JSON instead of writing files |

## JSON output

```json
[
  {
    "index": 0,
    "file_offset": 0,
    "pcir_offset": 256,
    "vendor_id": 4098,
    "device_id": 26591,
    "class_code": 196608,
    "class_name": "Display controller",
    "declared_size_bytes": 60416,
    "code_type": 0,
    "code_type_name": "x86 legacy (PC-AT compatible)",
    "is_last_image": false,
    "is_atom_bios": true,
    "identity_string": null,
    "pcir_struct_length": 44
  },
  {
    "index": 1,
    "file_offset": 60416,
    "pcir_offset": 60672,
    "vendor_id": 4098,
    "device_id": 26591,
    "class_code": 196608,
    "class_name": "Display controller",
    "declared_size_bytes": 58368,
    "code_type": 3,
    "code_type_name": "EFI Image (typically UEFI GOP)",
    "is_last_image": true,
    "is_atom_bios": false,
    "identity_string": "EFI_AMD_GOP",
    "pcir_struct_length": 44
  }
]
```

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success - all images extracted |
| 1 | Error (could not read ROM, no images found, write failed) |
