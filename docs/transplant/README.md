# `transplant` - PCI ROM image swapping

Transplant PCI ROM images (Legacy/EFI) between VBIOS files with safety
guardrails: size checks, device-ID warnings, PCI header validation
and optional dry-run.

## Usage

```sh
# Transplant EFI (UEFI GOP) image only
polaris-vbios transplant target.rom --from donor.rom --out patched.rom --efi

# Transplant Legacy (x86) image only
polaris-vbios transplant target.rom --from donor.rom --out patched.rom --legacy

# Transplant both Legacy and EFI images
polaris-vbios transplant target.rom --from donor.rom --out patched.rom --both

# Dry-run (show plan without writing)
polaris-vbios transplant target.rom --from donor.rom --out patched.rom --efi --dry-run
```

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `target` | Yes | Target ROM (receives the transplanted image) |
| `--from` | Yes | Donor ROM (provides the image to transplant) |
| `--out` | Yes | Output ROM file (required - the source is never modified) |

## Flags

| Flag | Description |
|------|-------------|
| `--efi` | Transplant the EFI (UEFI GOP) image only |
| `--legacy` | Transplant the Legacy (x86) image only |
| `--both` | Transplant both Legacy and EFI images |
| `--target-index` | Image index in the target chain (default: auto-detect by type) |
| `--donor-index` | Image index in the donor chain (default: auto-detect by type) |
| `--dry-run` | Show the transplant plan and verification, write nothing |
| `--force` | Ignore device-ID mismatch warnings |

## Safety Guardrails

The transplant command validates several safety checks before proceeding:

1. **Size validation**: Donor image must not be larger than target image
   (cannot expand without corrupting the PCI ROM chain)
2. **Device-ID warnings**: Warns if donor and target device IDs differ
   (can override with `--force`)
3. **Vendor-ID warnings**: Warns if donor and target vendor IDs differ
4. **ATOM signature validation**: Validates Legacy images contain valid
   ATOM BIOS signatures
5. **PCI header validation**: Validates PCIR structure integrity
6. **Checksum validation**: Recalculates checksum for Legacy transplants

## PCI ROM Chain

AMD Polaris VBIOS files contain a PCI option ROM chain with two images:

- **Legacy** (code_type 0x00): x86 ROM for legacy BIOS
- **EFI** (code_type 0x03): EFI ROM for UEFI firmware (typically GOP)

The transplant command preserves the PCI ROM chain structure by:
- Maintaining the original indicator byte (last image flag)
- Padding with 0xFF if donor is smaller than target
- Preserving image_len_units from the donor

## Examples

### Transplant EFI from working ROM

```sh
# Your motherboard doesn't like the EFI in your ROM
# Use the EFI from a known-working ROM instead
polaris-vbios transplant 257447.rom \
  --from XFX.RX570.4096.170419.rom \
  --out 257447_patched.rom \
  --efi
```

### Dry-run to preview changes

```sh
# See what would happen without writing
polaris-vbios transplant 257447.rom \
  --from XFX.RX570.4096.170419.rom \
  --out 257447_patched.rom \
  --efi \
  --dry-run
```

### Transplant with device-ID override

```sh
# Force transplant even if device IDs differ
polaris-vbios transplant target.rom \
  --from donor.rom \
  --out patched.rom \
  --efi \
  --force
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error (validation failed, file I/O error, etc.) |
| 2 | Success but with warnings |

## Limitations

- Cannot expand the ROM size (donor must be smaller or equal to target)
- Validates structural consistency only (checksum, PCIR, sizes)
- Cannot confirm the result will POST on real hardware
- Final validation is always to flash and test
