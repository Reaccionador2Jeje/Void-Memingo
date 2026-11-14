# Void Boot — architecture and roadmap

## Product shape

Void Boot is a standalone Atdev Dev project and a first-class Void Memingo client. Both use shared Rust core capabilities.

    apps/
      void-boot/
        src/                 # CLI and shared Rust core
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
- **Desktop manager:** future standalone GUI for catalog, profiles, validation and media preparation.
- **Void Memingo client:** project-aware ISO inspection, TOML profile loading and generated GRUB menu preview using the shared Rust core.
- **Test harness:** QEMU virtual machines, disposable virtual disks and a compatibility matrix.\n- **Installer package:** CI-generated ZIP with shell, PowerShell and browser-based folder deployment entry points. The current package copies menu/theme/profile files only; it is not a bootable USB writer.

## Graphical boot experience

The boot-time GUI is not a desktop window: it runs before the operating system as GRUB's graphical gfxterm menu. Void Boot supplies a branded theme and generates menu entries from reviewed profiles. The pre-boot interface must remain usable without Linux, Windows or a running Void Memingo instance.

The first generated menu supports GRUB loopback entries for profiles that explicitly provide the correct ISO path, kernel path, initrd path and kernel arguments. Debian Live has a general amd64 recipe in `apps/void-boot/profiles/debian-live-loopback.toml`, using the documented `/live/vmlinuz`, `/live/initrd.img`, `boot=live` and `findiso` convention. A separate QEMU profile adds a serial console for observable end-to-end testing. The manual `debian-live-integration` workflow downloads the pinned Debian Live 13.7.0 standard ISO, checks its SHA-256 against Debian's published `SHA256SUMS`, places it on a disposable virtual FAT disk, and attempts a full boot through Void Boot in QEMU. This heavyweight test is manual-only; a successful GRUB menu smoke test is not a substitute for a successful Linux userspace boot. Generic loopback is not universal. Profiles must be validated against exact ISO releases, and unsupported images must be labelled as such rather than offered as working boot choices.

## Boot strategy

Use a mature bootloader rather than inventing firmware startup code. GRUB 2 is the initial candidate for x86_64 BIOS and UEFI. Secure Boot is deferred until a signing and trust-chain design is tested. BIOS and UEFI require separate installation and QEMU test results.

## Media safety

The current CLI and menu generator are read-only. They do not install GRUB, format, partition or write physical devices.

Before a physical-media writer is enabled, it must enumerate devices using stable identifiers, show model/capacity/mount state, refuse the active system disk, display the exact destructive plan, require a separate immediate confirmation, and verify the written result. The user's existing Ventoy USB must remain untouched throughout development.

## Current milestones

1. **Foundation:** ISO inventory, SHA-256, ISO 9660 and El Torito checks.
2. **Boot-menu foundation:** validated profile schema, GRUB menu generator, graphical theme, bundled Unicode font, ISO discovery and tests.
3. **Virtual boot prototype:** bootable GRUB menu ISO plus serial-console smoke tests for BIOS and UEFI using QEMU/OVMF; full graphical screenshot verification and disposable virtual-disk tests remain follow-up work.
4. **Profile coverage:** Debian Live recipe and manual full-boot QEMU test are implemented; confirm the test result before marking Debian verified, then add reviewed Ubuntu, Arch and rescue-image profiles against exact releases.
5. **Void Memingo integration:** shared profile catalog, ISO compatibility display, TOML profile picker and generated menu preview.
6. **Standalone GUI:** ISO library, profile editor, boot menu preview and safe media planner.
7. **Installer package:** ZIP distribution with shell, PowerShell and web folder installers; menu/profile deployment only.\n8. **Media writer:** guarded USB preparation only after virtual testing and explicit user approval.
10. **Compatibility expansion:** persistence, more distributions, Windows installer support where technically and legally appropriate.
11. **Secure Boot:** after signing, key management and trust requirements are fully designed.

## Acceptance criteria

- ISO inspection is deterministic and reports SHA-256.
- Profile parsing rejects path traversal and unsafe GRUB input.
- Generated menu clearly distinguishes examples from verified compatibility.
- Every supported image is tested in BIOS and UEFI separately.
- No test requires modifying a user's physical USB drive.
- Void Memingo and the standalone manager consume the same profile contracts.
