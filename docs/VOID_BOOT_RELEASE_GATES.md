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
- [x] Select a local GNU/BSD publisher checksum manifest before download and load the digest for the exact ISO filename; reject missing or conflicting entries.
- [x] Restrict ISO downloads to HTTPS hosts that resolve only to public IP addresses; pin the validated DNS results for the transfer and reject unsafe IP ranges.
- [x] Validate every redirect and allow only the original verified HTTPS host and port, preventing redirects to local or private services. Direct download URLs are required when an upstream page redirects to a separate CDN host.
- [x] Add pinned online checksum-manifest adapters for Debian stable amd64 ISO CD, Arch Linux current x86_64, Ubuntu 24.04 LTS amd64, Fedora Workstation 45 x86_64 and openSUSE Tumbleweed current DVD x86_64; require exact ISO filename matches and reject unknown distributions, redirects, oversized manifests and non-public DNS results.

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
- [ ] Verify downloaded images using authenticated upstream checksum/signature metadata. The current online adapters retrieve SHA-256 manifests over pinned HTTPS sources; they do not yet authenticate detached signatures, so they establish integrity against the fetched manifest but not publisher authenticity.

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


## Smart Download

- [x] Descarga paralela de hasta 8 rangos cuando el servidor admite HTTP Range.
- [x] Reanudación de partes descargadas en mirrors compatibles con rangos.
- [x] Fallback automático a una conexión simple cuando el mirror ignora HTTP Range.
- [x] SHA-256 calculado antes de finalizar el archivo; si se proporciona un hash esperado, debe coincidir.
- [ ] Selección automática de mirrors oficiales alternativos y biblioteca con detección de ISO reutilizables.
- [ ] Descarga directa a medio USB; requiere primero el flujo seguro de detección, confirmación y escritura del dispositivo.

Nota: el modo de conexión simple no puede reanudar una transferencia interrumpida en servidores que no admiten rangos; al volver a iniciarse, comienza de nuevo. La velocidad depende de la conexión, el servidor y sus límites, por lo que el paralelismo no garantiza aceleración.
