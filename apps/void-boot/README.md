# Void Boot

Void Boot is Atdev Dev's independent, open-source multiboot project and a first-class Void Memingo client.

## Current capabilities

- Read-only ISO inventory with SHA-256, ISO 9660 and El Torito descriptor inspection.
- Declarative boot profiles with validation of IDs, labels, relative paths and kernel arguments.
- GRUB 2 configuration generator for a pre-operating-system boot menu.
- Graphical GRUB theme with Void Boot branding, boot menu, restart and shutdown entries.
- CLI and reusable Rust library shared with Void Memingo.
- No physical drive is mounted, formatted, partitioned or written by these features.

The generated menu uses GRUB's graphical gfxterm menu and profile-specific loopback entries. **A profile is only a boot recipe, not proof that an ISO is compatible.** Kernel/initrd paths and arguments must match the exact image. Some ISOs need extraction or a custom boot method.

## Commands

Requires Rust stable.

    cd apps/void-boot
    cargo test
    cargo run -- scan /path/to/iso-folder
    cargo run -- scan /path/to/iso-folder --json
    cargo run -- render-menu profiles/example-profiles.toml /tmp/void-boot-menu

From the repository root, build a bootable graphical menu ISO for a virtual machine:

    bash scripts/build-void-boot-demo-iso.sh

CI also builds a serial-console test image and smoke-tests the menu using QEMU with legacy BIOS and OVMF UEFI:

    VOID_BOOT_SERIAL_TEST=1 bash scripts/build-void-boot-demo-iso.sh apps/void-boot/profiles/example-profiles.toml /tmp/void-boot-qemu.iso
    bash scripts/test-void-boot-qemu.sh /tmp/void-boot-qemu.iso

The render command writes grub.cfg and copies the graphical theme into the output folder. It does not install GRUB or write to a USB drive. Void Memingo can also open a profile TOML file and show the generated GRUB menu without writing it.

## Intended removable-media layout

    /boot/grub/grub.cfg
    /boot/grub/themes/void-boot/theme.txt
    /iso/<your-images>.iso

The generated configuration assumes GRUB's $prefix points to /boot/grub. A future media builder will install and configure GRUB for BIOS and UEFI separately. This stage does not yet create a bootable USB by itself.

## Safety and compatibility

Physical-device writing is not implemented. Development and compatibility testing starts with disposable virtual disks and QEMU. The user's existing Ventoy USB must remain untouched.

Secure Boot is deferred until the signing and trust chain are designed and tested. No image should be marked supported until its profile is tested independently in BIOS and UEFI virtual machines.
