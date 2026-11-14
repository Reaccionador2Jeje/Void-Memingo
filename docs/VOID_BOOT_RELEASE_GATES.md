# Void Boot release gates

Void Boot is not considered production-ready until every required gate below has evidence attached to the release.

## Current implemented foundation

- [x] Rust CLI and reusable library.
- [x] Read-only ISO inventory, SHA-256 and ISO 9660 / El Torito descriptor inspection.
- [x] Declarative profile parsing and input validation.
- [x] GRUB graphical menu generation and Atdev theme.
- [x] Void Memingo integration for ISO inspection and profile preview.
- [x] Linux read-only removable-device inventory.
- [x] QEMU BIOS and UEFI menu-start smoke-test automation.
- [x] Manual Debian Live full-handoff test harness.

## Required before a stable release

### 1. Boot compatibility
- [ ] Run and pass the Debian Live 13.7.0 QEMU full-boot workflow.
- [ ] Verify at least one supported image per supported distribution family against a pinned release.
- [ ] Verify each supported recipe in BIOS and UEFI, with logs and reproducible test inputs.
- [ ] Clearly classify profiles as tested, experimental or unsupported.
- [ ] Test missing/corrupt images, unusual filenames, large images and multiple storage devices.

### 2. Removable media
- [ ] Implement Windows and macOS read-only device discovery.
- [ ] Design a non-destructive media-preparation plan and preview.
- [ ] Add explicit device identity, capacity, mounted-state and system-disk safeguards.
- [ ] Require a separate confirmation step before any future destructive operation.
- [ ] Add interruption handling, verification and recovery documentation.
- [ ] Never select a device implicitly; never silently format or overwrite a drive.

### 3. Firmware and security
- [ ] Validate BIOS and UEFI media layout on real supported hardware.
- [ ] Design Secure Boot support and signing/trust chain separately.
- [ ] Audit GRUB configuration generation and profile input validation.
- [ ] Verify downloaded images using authenticated upstream checksum/signature metadata.

### 4. Product experience
- [ ] Ship a standalone desktop UI with device inventory, ISO library, profile manager and build logs.
- [ ] Keep Void Memingo integration and standalone Void Boot on the same library/API.
- [ ] Provide clear error messages, accessibility, localization and recovery guidance.
- [ ] Produce signed/reproducible release packages for supported desktop platforms.

### 5. Release engineering
- [ ] All Rust tests, Clippy and formatting checks pass.
- [ ] Desktop frontend and Tauri backend checks pass.
- [ ] BIOS/UEFI QEMU tests pass.
- [ ] Full Linux handoff tests pass and artifacts are retained.
- [ ] Document supported systems, limitations, upgrade and recovery behavior.
- [ ] Independent review of media-handling safeguards.

## Safety boundary during development

Until the media builder has passed the safeguards and test gates, Void Boot only inventories removable devices and generates boot menus. It does not partition, format, install a bootloader to, or write to physical drives. Use disposable virtual disks for development and QEMU testing.
