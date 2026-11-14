# Void Boot — architecture and roadmap

## Product shape

Void Boot is Atdev Dev's GUI-only multiboot manager, delivered as a first-class graphical workspace inside Void Memingo. The shared Rust core is an internal engine, not a user-facing CLI or TUI.

    apps/
      void-boot/
        src/                 # Shared Rust engine and domain logic
        profiles/            # reviewed per-ISO boot recipes
        assets/grub/         # graphical pre-boot theme
      desktop/
        src/                 # Void Memingo integration
    docs/
      VOID_BOOT_ARCHITECTURE.md

## Components

- **ISO catalog:** file metadata, SHA-256, ISO 9660 and El Torito inspection.
- **Boot profiles:** per-image kernel/initrd paths, kernel arguments, optional live-boot ISO rediscovery and compatibility notes.
- **GRUB menu generator:** creates a graphical pre-OS menu and profile-driven entries.
- **Media planner:** future dry-run partition/filesystem layout.
- **Boot runtime:** GRUB 2 for initial BIOS/UEFI prototype; replaceable behind a runtime boundary.
- **Desktop manager:** graphical Void Boot workspace for catalog, ISO library, downloads, checksum verification, profile selection/preview and virtual testing.
- **Void Memingo GUI:** all Void Boot user workflows are exposed through graphical panels, dialogs, file pickers and progress indicators; no CLI/TUI is offered as a user interface.
- **Test harness:** QEMU virtual machines, disposable virtual disks and a compatibility matrix.
- **Build pipeline:** CI-only scripts assemble and validate disposable boot-menu images; these scripts are developer automation, not a Void Boot user interface.

## Graphical boot experience

The boot-time GUI is not a desktop window: it runs before the operating system as GRUB's graphical gfxterm menu. Void Boot supplies a branded theme and generates menu entries from reviewed profiles. The pre-boot interface must remain usable without Linux, Windows or a running Void Memingo instance.

The first generated menu supports GRUB loopback entries for profiles that explicitly provide the correct ISO path, kernel path, initrd path and kernel arguments. Debian Live has a general amd64 recipe in `apps/void-boot/profiles/debian-live-loopback.toml`, using the documented `/live/vmlinuz`, `/live/initrd.img`, `boot=live` and `findiso` convention. A separate QEMU profile adds a serial console for observable end-to-end testing. The manual `debian-live-integration` workflow downloads the pinned Debian Live 13.7.0 standard ISO, checks its SHA-256 against Debian's published `SHA256SUMS`, places it on a disposable virtual FAT disk, and attempts a full boot through Void Boot in QEMU under both legacy BIOS and OVMF UEFI. This heavyweight test is manual-only; a successful GRUB menu smoke test is not a substitute for a successful Linux userspace boot. Generic loopback is not universal. Profiles must be validated against exact ISO releases, and unsupported images must be labelled as such rather than offered as working boot choices.

## Boot strategy

Use a mature bootloader rather than inventing firmware startup code. GRUB 2 is the initial candidate for x86_64 BIOS and UEFI. Secure Boot is deferred until a signing and trust-chain design is tested. BIOS and UEFI require separate installation and QEMU test results.

## Media safety

The graphical ISO inspection and menu preview are read-only. They do not install GRUB, format, partition or write physical devices.

Before a physical-media writer is enabled, it must enumerate devices using stable identifiers, show model/capacity/mount state, refuse the active system disk, display the exact destructive plan, require a separate immediate confirmation, and verify the written result. Physical USB devices remain untouched during development and virtual-image testing.

## Current milestones

1. **Foundation:** ISO inventory, SHA-256, ISO 9660 and El Torito checks.
2. **Boot-menu foundation:** validated profile schema, GRUB menu generator, graphical theme, bundled Unicode font, ISO discovery and tests.
3. **Virtual boot prototype:** bootable GRUB menu ISO with serial-console smoke tests in QEMU for legacy BIOS and OVMF UEFI. A disposable GPT/FAT32 media image is boot-tested in QEMU under OVMF UEFI; BIOS media-image boot remains pending.
4. **Profile coverage:** Debian Live recipe and manual full-boot QEMU test are implemented; confirm the test result before marking Debian verified, then add reviewed Ubuntu, Arch and rescue-image profiles against exact releases.
5. **Void Memingo integration:** shared profile catalog, ISO compatibility display, visual profile editor with validated TOML load/save, generated menu preview and QEMU virtual testing.
6. **Standalone GUI:** ISO library, profile editor, boot menu preview and safe media planner.
7. **Installer package:** ZIP distribution with shell, PowerShell and web folder installers; menu/profile deployment only.
8. **Virtual media validation:** disposable GPT/FAT32 media images, multi-ISO library import and OVMF UEFI boot checks run in CI; BIOS media-image boot remains pending.
9. **Media writer:** guarded USB preparation only after virtual testing and explicit user approval.
10. **Compatibility expansion:** persistence, more distributions, Windows installer support where technically and legally appropriate.
11. **Secure Boot:** after signing, key management and trust requirements are fully designed.

## Acceptance criteria

- ISO inspection is deterministic and reports SHA-256.
- Profile parsing rejects path traversal and unsafe GRUB input.
- Generated menu clearly distinguishes examples from verified compatibility.
- Every supported image is tested in BIOS and UEFI separately.
- No test requires modifying a user's physical USB drive.
- Void Memingo and the standalone manager consume the same profile contracts.
