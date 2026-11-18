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
- [x] Discover Arch Linux HTTPS mirrors from the official mirror-status JSON API; reject inactive, low-completion, stale, non-HTTPS or malformed entries, prefer Chile, and retain download-time HTTPS/DNS/public-IP validation.
- [x] Build Debian ISO download candidates on Debian's documented CloudFront-backed `/debian-cd/` endpoint; accept only safe relative `.iso` paths and keep the URL pinned to the official HTTPS CDN.
- [x] Query Kali's official `cdimage.kali.org` redirector for an exact ISO and parse its advertised HTTPS redirect and duplicate-mirror candidates; reject unsafe paths, non-HTTPS URLs and mismatched artifacts.
- [x] Resolve Fedora Workstation x86_64 ISO requests through Fedora's official `download.fedoraproject.org` redirector; pin the redirector DNS result, reject non-HTTPS destinations, credentials, query strings and artifact-path mismatches.
- [x] Build official Ubuntu, Lubuntu, Kubuntu and Xubuntu ISO destinations from an explicit release and exact filename; use Canonical's documented `releases.ubuntu.com` or `cdimage.ubuntu.com` host without pretending these fixed endpoints are a multi-mirror selector.
- [x] Query openSUSE's official per-artifact `.mirrorlist` page; parse only HTTPS links with the exact requested ISO path, reject duplicate/mismatched/non-HTTPS entries, and cap the returned list.
- [x] Read the official OpenBSD and NetBSD mirror directories and build exact-artifact HTTPS candidates only from the publishers' documented mirror roots; reject unsafe paths and non-HTTPS sources.
- [x] Register mirror/download-source capabilities for every catalogue entry: Arch live API, Debian official image CDN, Kali image redirector, Fedora MirrorManager, openSUSE Metalink, FreeBSD GeoDNS, OpenBSD/NetBSD mirror directories, and project-specific official download sources for remaining Linux, BSD and Windows entries. Capability labels distinguish selectable lists from documentation/CDN/direct-download sources.
- [x] Enforce a global Distro Center download release gate in both the UI and Tauri backend; no distribution can start a download or open its download-source link until every catalogue system passes release validation.

## Required before a stable release

### 0. Global Distro Center download gate
- [ ] Complete and validate every system in the 40-entry catalogue before enabling any download.
- [ ] For each system, provide an official artifact resolver, authenticated integrity verification, a tested boot/install profile where applicable, and BIOS/UEFI evidence or an explicit documented platform exception.
- [ ] Update the release-gate implementation from locked to enabled only after all evidence is reviewed. The current backend intentionally reports 0/40 validated and rejects every download.

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
- [x] Registro de capacidad de mirrors/fuentes para todo el catálogo Linux, BSD y Windows, con etiquetas distintas para API seleccionable, directorio oficial, MirrorManager, Metalink, GeoDNS/CDN y descarga oficial directa.
- [x] Selector dinámico real para Arch Linux: API oficial, solo HTTPS activo, completitud ≥99%, retraso ≤6 horas y prioridad de Chile.
- [x] Resolver de Debian: genera el destino de ISO usando el CDN oficial `cloudfront.debian.net/debian-cd/`, con validación de ruta relativa; el CDN global no se presenta como selector de múltiples mirrors.
- [x] Resolver de Kali: consulta el redirector oficial para obtener el destino y los mirrors HTTPS alternativos anunciados para la ISO exacta.
- [x] Resolver de Fedora Workstation x86_64: consulta el redirector oficial y valida el destino HTTPS anunciado para el archivo exacto.
- [x] Resolver de Ubuntu, Lubuntu, Kubuntu y Xubuntu: genera destinos de ISO en los hosts oficiales a partir de versión y nombre de archivo exactos; no lo presenta como selección de mirrors.
- [x] Resolver de openSUSE: consulta la página oficial `.mirrorlist` para un artefacto exacto y filtra los mirrors HTTPS anunciados.
- [x] Resolver de OpenBSD y NetBSD: extrae mirrors HTTPS de los directorios oficiales y construye el destino para la ruta relativa exacta de ISO indicada.
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


### Mirror and source coverage

Void Boot now classifies every catalogue entry rather than showing the Arch selector as if it applied to all systems. Arch currently has a live, selectable ISO mirror list. Debian has a safe ISO path resolver pinned to its official CloudFront-backed image CDN, and Kali queries its official image redirector for the chosen ISO and advertised HTTPS alternatives. Fedora, openSUSE, FreeBSD, OpenBSD and NetBSD expose their publisher's documented mirror infrastructure or official GeoDNS/Metalink entry point, but the release-specific ISO resolver is not yet implemented. Ubuntu flavors and other derivatives use their own official download channels unless an ISO-specific mirror source is verified. Windows uses Microsoft download pages only; third-party Windows mirrors are not offered. These entries are source guidance, not claims that automatic mirror selection is complete.
