# Void Boot

Void Boot is Atdev Dev's independent, open-source multiboot project and a first-class Void Memingo client.

## Current capabilities

- Read-only ISO inventory with SHA-256, ISO 9660 and El Torito descriptor inspection.
- Declarative boot profiles with validation of IDs, labels, ISO/kernel/initrd paths, argument limits and duplicate IDs.
- `validate-profiles` command for checking profile catalogs before generating a menu.
- Read-only Linux removable-device inventory showing model, capacity, read-only state and whether the device is mounted. This command never opens a device for writing.
- GRUB 2 configuration generator for a pre-operating-system boot menu.
- Graphical GRUB theme with Void Boot branding, boot menu, restart and shutdown entries.
- CLI and reusable Rust library shared with Void Memingo.
- No physical drive is mounted, formatted, partitioned or written by these features.

The generated menu uses GRUB's graphical gfxterm menu and profile-specific loopback entries. **A profile is only a boot recipe, not proof that an ISO is compatible.** Kernel/initrd paths and arguments must match the exact image. Some ISOs need extraction or a custom boot method.

## Distribution Center (catalogue milestone)

Void Boot includes a catalogue of official project and download pages for Debian, Ubuntu, Fedora, Arch Linux, Linux Mint and Zorin OS. View it with `void-boot catalog` (also `list-distributions`). Open a listed download page, choose the edition and architecture, and save the ISO into your local library folder. Then inspect it with `void-boot scan /path/to/isos`.

The catalogue points to official landing pages rather than versioned mirror URLs, avoiding stale download links. **This milestone does not download ISO files automatically, verify publisher signatures, or mark any distro as boot-compatible.** Follow each project's verification instructions and compare its published checksum/signature before using an image. A future downloader must implement HTTPS-only transfers, publisher-authenticated verification, resumable downloads and clear consent before large downloads.

## Commands

Requires Rust stable.

    cd apps/void-boot
    cargo test
    cargo run -- catalog\n    cargo run -- scan /path/to/iso-folder
    cargo run -- scan /path/to/iso-folder --json
    cargo run -- list-devices
    cargo run -- list-devices --json
    cargo run -- validate-profiles profiles/example-profiles.toml
    cargo run -- render-menu profiles/example-profiles.toml /tmp/void-boot-menu

From the repository root, build a bootable graphical menu ISO for a virtual machine:

    bash scripts/build-void-boot-demo-iso.sh

CI also builds a serial-console test image and smoke-tests the menu using QEMU with legacy BIOS and OVMF UEFI:

    VOID_BOOT_SERIAL_TEST=1 bash scripts/build-void-boot-demo-iso.sh apps/void-boot/profiles/example-profiles.toml /tmp/void-boot-qemu.iso
    bash scripts/test-void-boot-qemu.sh /tmp/void-boot-qemu.iso

The render command writes grub.cfg and copies the graphical theme into the output folder. It does not install GRUB or write to a USB drive. Void Memingo can also open a profile TOML file and show the generated GRUB menu without writing it.


## UEFI media image builder (virtual image only)

Build a GPT/FAT32 UEFI image containing the standalone GRUB EFI loader and one or many ISO payloads:

    sudo apt-get install grub-efi-amd64-bin gdisk mtools dosfstools
    # Single ISO (legacy mode):
    bash scripts/build-void-boot-media.sh apps/void-boot/profiles/debian-live-loopback.toml /path/to/debian.iso target/void-boot-media.img

    # ISO library mode: every distinct ISO path in the profile catalog is imported.
    # Put source files directly in /path/to/isos, named by each profile's ISO basename.
    bash scripts/build-void-boot-media.sh apps/void-boot/profiles/library-profiles.toml /path/to/isos target/void-boot-library.img

In library mode, the profile catalog is the import manifest: each `iso = "iso/name.iso"` entry maps to `/path/to/isos/name.iso`. Every referenced file must exist, be non-empty and not be a symlink. Duplicate basenames mapped to different ISO paths are rejected. The builder creates the destination directories, copies each unique ISO, then extracts and compares every copy byte-for-byte. The generated GRUB menu contains one entry per profile. Add a profile only when its kernel/initrd paths and boot arguments match that exact ISO; unsupported or untested profiles must not be treated as verified.

Set `VOID_BOOT_IMAGE_SIZE_MIB` to change the image size (minimum 512 MiB; the default is 1024 MiB). Choose enough capacity for all ISO files plus filesystem overhead. The output path must not already exist.

This milestone creates a **UEFI-only virtual disk image**. It does not write to USB devices, install BIOS boot code, or claim that every Linux ISO supports loopback boot. BIOS support and QEMU firmware validation remain separate gates.

## ZIP installer package

The Void Boot CI workflow builds a `void-boot-installer-zip` artifact. It contains the generated GRUB menu, theme, profile catalogues, and three installation entry points:

- `install.sh` — Linux/macOS terminal installer.
- `install.ps1` — Windows PowerShell installer.
- `web-installer.html` — browser-based folder copier for supported browsers.

To build the package locally from the repository root (requires Rust stable and `zip`):

    bash scripts/package-void-boot.sh

To install from an extracted ZIP, run `./install.sh` or `./install.ps1`, or open `web-installer.html` and select the extracted `Void-Boot` folder followed by a destination directory.

**Important:** this first package deploys menu, theme and profile files only. It does not install GRUB, create EFI boot files, partition or format storage, or make a USB bootable. A real BIOS/UEFI USB installer remains a separate milestone and must pass disposable virtual-disk tests before physical-media support.

## Intended removable-media layout

    /boot/grub/grub.cfg
    /boot/grub/themes/void-boot/theme.txt
    /iso/<your-images>.iso

The generated configuration assumes GRUB's $prefix points to /boot/grub. A future media builder will install and configure GRUB for BIOS and UEFI separately. This stage does not yet create a bootable USB by itself.

## Safety and compatibility

Physical-device writing and a standalone graphical device manager are not implemented. Removable-device discovery currently supports Linux; Windows and macOS discovery adapters remain future work. Development and compatibility testing starts with disposable virtual disks and QEMU. The user's existing Ventoy USB must remain untouched. Do not treat the current menu ISO as a finished USB installer.

Secure Boot is deferred until the signing and trust chain are designed and tested. No image should be marked supported until its profile is tested independently in BIOS and UEFI virtual machines.

## Release readiness

See [Void Boot release gates](../../docs/VOID_BOOT_RELEASE_GATES.md) for the requirements that must pass before a stable release.
