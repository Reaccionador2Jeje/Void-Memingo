# Void Boot — architecture and roadmap

## Product shape

Void Boot is a standalone Atdev Dev project and a first-class Void Memingo client. Both use shared Rust core capabilities; neither should duplicate ISO parsing, validation, profile definitions, or media layout logic.

Proposed repository layout:

```text
apps/
  void-boot/
    src/                 # standalone CLI and future desktop shell
    core/                # shared ISO, profile, manifest and media logic (planned)
    profiles/            # reviewed boot profiles (planned)
    assets/              # menu theme and icons (planned)
  desktop/
    src/features/void-boot/  # Void Memingo feature (planned)
docs/
  VOID_BOOT_ARCHITECTURE.md
```

The current implementation is a small standalone CLI in `apps/void-boot/src/main.rs`. The future shared core should move into a library crate once the first profile and media contracts are stable.

## Components

- **ISO catalog:** file metadata, SHA-256, ISO 9660 and El Torito descriptor inspection.
- **Boot profiles:** explicit per-distribution configuration describing supported firmware modes, kernel/initrd strategy, required files, boot arguments and known limitations.
- **Media planner:** calculates a proposed partition/filesystem layout without changing a device.
- **Boot runtime:** menu and bootloader configuration for BIOS/UEFI.
- **Desktop manager:** add/remove ISO entries, inspect compatibility, verify hashes and prepare media.
- **Void Memingo client:** project-aware access to the same catalog and build artifacts.
- **Test harness:** QEMU virtual machines, disposable virtual disks and a compatibility matrix.

## Boot strategy

Do not attempt to invent a firmware bootloader. Evaluate a mature bootloader (GRUB2 is the initial candidate) and keep the boot-runtime boundary replaceable. The first proof of concept targets x86_64 BIOS and UEFI without Secure Boot. Secure Boot requires a separate signing and trust-chain design.

ISO boot is not universally interchangeable. Some distributions support loopback boot, while others require extraction, custom kernel/initrd handling, or image-specific configuration. Every supported ISO family therefore needs a reviewed profile and repeatable QEMU tests. A profile must state its supported release range and known limitations.

## Media safety

The initial and current CLI is read-only. No physical-drive writing, formatting, partitioning, or automatic device selection is included.

Before any future physical-media writer is enabled, it must:
- Enumerate devices using stable identifiers and display model, capacity and mount state.
- Require an explicit target chosen by the user; never guess based on removable status alone.
- Refuse the running system disk and any device containing the active workspace.
- Present the exact partition plan and explain that existing contents will be erased.
- Require a second, immediate confirmation after the final device identity is re-read.
- Verify written files and boot structures afterward.
- Keep a dry-run planner and QEMU virtual-disk test path.

The user's existing Ventoy USB must remain untouched during development.

## Milestones

1. **Foundation (in progress):** standalone CLI, ISO inventory, SHA-256, ISO 9660 and El Torito descriptor checks.
2. **Profiles:** declarative profiles for selected Debian, Ubuntu and Arch images; profile validation and test fixtures.
3. **Virtual boot prototype:** generated boot configuration and QEMU tests using disposable virtual disks.
4. **Void Memingo integration:** initial read-only ISO scanner is integrated in the IDE; shared profile catalog and advanced management UI remain pending.
5. **USB media planner:** device discovery and dry-run partition layout only.
6. **Media writer:** explicit, guarded writing after extensive virtual testing.
7. **Compatibility expansion:** more distributions, persistence, rescue tools, Windows installer support where technically and legally appropriate.
8. **Secure Boot:** only after signing, key management and trust requirements are fully designed.

## Acceptance criteria

- ISO inspection is deterministic and reports a SHA-256 digest.
- Unknown or unsupported images are labelled honestly rather than guessed.
- Boot profiles are versioned and tested in QEMU before being marked supported.
- BIOS and UEFI results are recorded separately.
- No test requires modifying a user's physical USB drive.
- Void Memingo and the standalone manager consume the same profile and catalog contracts.
