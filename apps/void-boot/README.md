# Void Boot

Void Boot is Atdev Dev's independent, open-source multiboot project. It will have two front ends backed by shared components:

1. **Standalone application** — planned desktop manager using the shared Rust core.
2. **Void Memingo integration** — the first read-only ISO inspector is already available in the desktop activity bar; profile management and media preparation remain planned.

The project is intentionally separate from the Void Memingo desktop UI so it can evolve and ship independently.

## Current status

The first safe foundation is an ISO inventory CLI:

- Finds ISO files directly inside a selected directory (no recursive traversal).
- Calculates SHA-256 using streaming reads.
- Checks for the ISO 9660 primary volume descriptor.
- Detects the El Torito boot-record descriptor when present.
- Supports human-readable and JSON output.
- Is integrated into Void Memingo as a read-only ISO inspector using the same Rust library.
- Does not mount, format, partition, write to, or otherwise modify storage devices.

This is an **inspection tool**, not yet a bootloader, USB installer, or Ventoy replacement.

## Build and run

Requires Rust stable.

```sh
cd apps/void-boot
cargo test
cargo run -- scan /path/to/iso-folder
cargo run -- scan /path/to/iso-folder --json
```

## Safety boundary

Physical-device writing is not implemented. Future media-writing features must require explicit device selection, show the detected model/capacity and destructive consequences, reject the active system disk, and require a separate confirmation immediately before writing. Development and compatibility testing starts with disposable virtual disks and QEMU.

## Planned boot support

Initial target: x86_64 PC firmware, BIOS and UEFI, with Secure Boot deferred until the trust/signing chain is designed and tested. ISO boot compatibility is profile-based; not every Linux ISO supports generic loopback boot. The project must never promise universal compatibility without per-image tests.

See [architecture](../../docs/VOID_BOOT_ARCHITECTURE.md) for design and milestones.
