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
- [x] Read antiX's publisher-maintained download page over pinned HTTPS and extract a SHA-256 only when the exact ISO filename has one unique 64-hex digest; reject missing or conflicting entries. This adapter is available for checksum lookup but is not yet enabled for automated downloads.
- [x] Resolve Rocky Linux 9/10 x86_64 ISO names to the official per-major-version `CHECKSUM` file, reject unsupported versions/architectures/image types, and require one exact SHA-256 filename match. This adapter is not yet enabled for automated downloads.
- [x] Discover Arch Linux HTTPS mirrors from the official mirror-status JSON API; reject inactive, low-completion, stale, non-HTTPS or malformed entries, prefer Chile, and retain download-time HTTPS/DNS/public-IP validation.
- [x] Build Debian ISO download candidates on Debian's documented CloudFront-backed `/debian-cd/` endpoint; accept only safe relative `.iso` paths and keep the URL pinned to the official HTTPS CDN.
- [x] Query Kali's official `cdimage.kali.org` redirector for an exact ISO and parse its advertised HTTPS redirect and duplicate-mirror candidates; reject unsafe paths, non-HTTPS URLs and mismatched artifacts.
- [x] Resolve Fedora Workstation x86_64 ISO requests through Fedora's official `download.fedoraproject.org` redirector; pin the redirector DNS result, reject non-HTTPS destinations, credentials, query strings and artifact-path mismatches.
- [x] Parse Mageia's official mirror database and retain only announced HTTPS mirror roots for exact ISO paths under `iso/`.
- [x] Parse the MX Linux publisher-maintained mirmon status page and retain only its advertised HTTPS ISO mirror roots for exact `MX/Final/` or `MX/Snapshots/` artifact paths.
- [x] Resolve Void Linux live installer ISO paths only inside the official `repo-default.voidlinux.org/live/current/` tree; use the publisher's default endpoint without claiming a selectable mirror list.
- [x] Build official Ubuntu, Lubuntu, Kubuntu and Xubuntu ISO destinations from an explicit release and exact filename; use Canonical's documented `releases.ubuntu.com` or `cdimage.ubuntu.com` host without pretending these fixed endpoints are a multi-mirror selector.
- [x] Query openSUSE's official per-artifact `.mirrorlist` page; parse only HTTPS links with the exact requested ISO path, reject duplicate/mismatched/non-HTTPS entries, and cap the returned list.
- [x] Read the official OpenBSD and NetBSD mirror directories and build exact-artifact HTTPS candidates only from the publishers' documented mirror roots; reject unsafe paths and non-HTTPS sources.
- [x] Build FreeBSD ISO destinations on the official `download.freebsd.org` GeoDNS/CDN service from an exact relative artifact path; do not represent the GeoDNS endpoint as a manual mirror list.
- [x] Read Linux Mint's official ISO mirror directory and filter only HTTPS links whose base path is the dedicated `/iso/` tree, excluding repository/package mirrors.
- [x] Resolve Slackware 15.0 32-bit and 64-bit installer DVD images only inside the publisher's official `slackware-iso/` index.
- [x] Parse exact HTTPS ISO links announced by the official download pages of Linux Lite, Solus and antiX; reject mismatched filenames, unsupported hosts where the publisher pins a domain, and non-HTTPS links.
- [x] Resolve exact ReactOS BootCD/LiveCD ISO paths only inside the official BuildBot archive; identify these artifacts as nightly builds.
- [x] Give each Windows catalogue entry its correct Microsoft download page handoff, including separate x64 and ARM64 pages and the distinct Windows 10 LTSC Evaluation page.
- [x] Expose the official source page as a clickable Distro Center link and add the ReactOS nightly ISO path picker to the desktop UI.
- [x] Register mirror/download-source capabilities for every catalogue entry: Arch live API, Debian official image CDN, Kali image redirector, Fedora MirrorManager, openSUSE Metalink, FreeBSD GeoDNS, OpenBSD/NetBSD mirror directories, and project-specific official download sources for remaining Linux, BSD and Windows entries. Capability labels distinguish selectable lists from documentation/CDN/direct-download sources.
- [x] Enforce a per-distribution download allowlist in the Tauri backend: only systems with an implemented official SHA-256 adapter and an explicit enablement entry can start automated downloads. Official source-page handoffs are a separate capability and do not imply direct-download or checksum coverage.

## Required before a stable release

### 0. Global Distro Center download gate
- [ ] Complete SHA-256 adapter coverage for all 40 catalogue entries. Automated downloads are currently enabled only for 8/40 systems; the other entries remain blocked until their official integrity source is implemented and reviewed.
- [ ] For each system, provide an official artifact resolver, an official integrity source, a tested boot/install profile where applicable, and BIOS/UEFI evidence or an explicit documented platform exception.
- [ ] Expand the backend allowlist only after the corresponding publisher checksum adapter and its tests are reviewed. The antiX adapter is implemented but intentionally not enabled yet.

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
- [ ] Verify downloaded images using authenticated upstream checksum/signature metadata. The current online adapters retrieve SHA-256 metadata over pinned HTTPS sources; they do not yet authenticate detached signatures, so they establish integrity against the fetched metadata but not publisher authenticity.

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


## Download engine foundations (Smart Download deferred)

The checked items below are existing low-level download and source-resolution capabilities. They do not represent the finished Smart Download product. Per the project roadmap, the real Smart Download orchestration—automatic release/architecture selection, mirror scoring and fallback, unified progress/retry/resume policy, integrity verification and library deduplication—must not be implemented or presented as complete until source integration for the full 40-system catalogue is finished.

- [x] Descarga paralela de hasta 8 rangos cuando el servidor admite HTTP Range.
- [x] Reanudación de partes descargadas en mirrors compatibles con rangos.
- [x] Fallback automático a una conexión simple cuando el mirror ignora HTTP Range.
- [x] SHA-256 calculado antes de finalizar el archivo; si se proporciona un hash esperado, debe coincidir.
- [x] Registro de capacidad de mirrors/fuentes para todo el catálogo Linux, BSD y Windows, con etiquetas distintas para API seleccionable, directorio oficial, MirrorManager, Metalink, GeoDNS/CDN y descarga oficial directa.
- [x] Selector dinámico real para Arch Linux: API oficial, solo HTTPS activo, completitud ≥99%, retraso ≤6 horas y prioridad de Chile.
- [x] Resolver de Debian: genera el destino de ISO usando el CDN oficial `cloudfront.debian.net/debian-cd/`, con validación de ruta relativa; el CDN global no se presenta como selector de múltiples mirrors.
- [x] Resolver de Kali: consulta el redirector oficial para obtener el destino y los mirrors HTTPS alternativos anunciados para la ISO exacta.
- [x] Resolver de Fedora Workstation x86_64: consulta el redirector oficial y valida el destino HTTPS anunciado para el archivo exacto.
- [x] Resolver de Ubuntu, Lubuntu, Kubuntu y Xubuntu: genera destinos de ISO en los hosts oficiales a partir de versión y nombre de archivo exactos; no lo presenta como selección de mirrors.
- [x] Resolver de openSUSE: consulta la página oficial `.mirrorlist` para un artefacto exacto y filtra los mirrors HTTPS anunciados.
- [x] Resolver de OpenBSD y NetBSD: extrae mirrors HTTPS de los directorios oficiales y construye el destino para la ruta relativa exacta de ISO indicada.
- [x] Resolver de FreeBSD: genera el destino de ISO HTTPS en el host oficial GeoDNS/CDN a partir de una ruta relativa validada.
- [x] Resolver de Linux Mint: obtiene mirrors ISO desde el directorio oficial y excluye las rutas de repositorios de paquetes.
- [ ] Completar Fedora con un selector de mirrors medidos y soporte de otras ediciones/arquitecturas; el resolver actual cubre solo Workstation x86_64 mediante el redirector oficial.
- [ ] Validar el resolver openSUSE MirrorCache con respuestas reales por arquitectura y tipo de imagen; medir disponibilidad y latencia de los mirrors antes de recomendarlos.
- [ ] Completar pruebas de integración OpenBSD/NetBSD con respuestas reales y comprobar que cada mirror conserva la ruta de imagen correcta por versión y arquitectura.
- [ ] Mejorar Debian con descubrimiento y medición de mirrors de imagen más allá de su CDN oficial.
- [ ] Integrar los índices propios de mirrors ISO de Ubuntu y derivados, Linux Mint y otros proyectos cuando se puedan verificar; no reutilizar mirrors de paquetes como si alojaran ISOs.
- [ ] Medir y comparar disponibilidad/latencia de mirrors sin tratar una lista estática o un CDN GeoDNS como un selector manual.
- [x] Reutilización de ISO ya presentes en la carpeta de biblioteca cuando se proporciona un SHA-256 esperado válido y el archivo existente coincide exactamente; se calcula el hash local y se registra como reutilización sin tráfico de descarga.
- [x] Rechazo seguro de reutilización cuando falta el SHA-256 esperado, el destino no es un archivo regular o el hash no coincide; el archivo existente nunca se sobrescribe.
- [ ] Descarga directa a medio USB; requiere primero el flujo seguro de detección, confirmación y escritura del dispositivo.

Nota: la caché no confía en el nombre ni en el tamaño como prueba de identidad. Para reutilizar, exige un SHA-256 esperado de una fuente que el usuario haya revisado; el resultado prueba coincidencia con ese valor, no autentica por sí solo al editor. El modo de conexión simple no puede reanudar una transferencia interrumpida en servidores que no admiten rangos; al volver a iniciarse, comienza de nuevo. La velocidad depende de la conexión, el servidor y sus límites, por lo que el paralelismo no garantiza aceleración.


### CI scheduling

The Void Boot CI runs once per pull-request update (plus manual dispatch), not once for both the branch push and the pull-request event. Each run has an independent concurrency group and does not cancel an in-progress validation. This avoids duplicate runner demand while preserving completed validation history.

### Mirror and source coverage

Void Boot classifies every catalogue entry rather than showing one distribution's selector as if it applied to all systems. Implemented artifact-source resolvers currently cover 35 of the 40 catalogue IDs: Arch (live mirror API), Debian (official CloudFront image CDN), Kali (official redirector), Fedora Workstation x86_64 (official redirector), Ubuntu/Lubuntu/Kubuntu/Xubuntu (publisher-specific official endpoints), openSUSE (per-artifact MirrorCache page), FreeBSD (official GeoDNS/CDN), OpenBSD and NetBSD (publisher mirror directories), Linux Mint (ISO-only mirror directory), Rocky Linux (official download host for versions 9 and 10), AlmaLinux (official CDN-backed master host), CentOS Stream (official Stream mirror host), Oracle Linux (official ISO host), Manjaro (official edition download host), EndeavourOS (live mirror links from its official download page), Zorin OS (the Kernel.org ISO mirror explicitly listed by Zorin), Pop!_OS (System76's official ISO host for amd64 generic/NVIDIA builds), elementary OS (live direct-link extraction from the official download page), Alpine Linux (official latest-stable ISO CDN tree), NixOS (official stable/unstable channel aliases for graphical/minimal x86_64 and aarch64 ISOs), Gentoo (live latest-build pointer from its official amd64 minimal ISO CDN index), Void Linux (official live/current image tree), MX Linux (publisher-maintained mirmon ISO mirror list), Mageia (official HTTPS mirror database), ReactOS (exact nightly BootCD/LiveCD paths in the official BuildBot ISO archive; not a stable release), KDE neon (exact live-image links from the official page), Slackware 15.0 (official 32-bit/64-bit ISO index), and Linux Lite, Solus and antiX (exact ISO links extracted from their official download pages). Some resolvers return a single official endpoint rather than a selectable mirror list. Fedora currently supports Workstation x86_64 only. The remaining 5 catalogue entries are the Windows variants. Their official page handoffs are now differentiated: Windows 11 x64, Windows 11 ARM64, the shared Windows 10 Home/Pro multi-edition ISO page, and the Windows 10 Enterprise LTSC Evaluation page. Microsoft generates signed, time-limited ISO URLs after the user selects an edition and language; these pages are not counted as permanent direct-artifact resolvers. Windows uses Microsoft download pages only; third-party Windows mirrors are not offered. ReactOS now has an exact-path resolver for its official BuildBot BootCD/LiveCD ISO archive, explicitly marked as nightly and potentially unstable rather than as the stable 0.4.16 release. The catalogue now has an automated test that iterates all 40 entries and fails if any system lacks an explicit HTTPS source policy. This is catalogue integration, not proof that all 40 have direct downloads. At this revision, 35/40 have artifact-source resolvers and 10/40 have built-in publisher-hosted SHA-256 adapters; 8/40 are currently enabled for automated downloads. Rocky Linux 9/10 x86_64 is resolved through the publisher's per-major-version CHECKSUM file and remains outside the enabled-download allowlist pending CI and release review. The antiX page adapter is implemented but remains outside the enabled-download allowlist until its release metadata and integration tests are reviewed. The remaining five Windows entries have official Microsoft page handoffs, but do not yet have permanent direct-artifact resolvers. The ReactOS resolver is limited to nightly images and does not certify a stable release. Download integrity and boot compatibility are separate release gates: a checksum adapter does not certify a boot profile, and a boot profile does not authenticate an ISO.
