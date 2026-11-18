use serde::Serialize;
use std::{io::Read, net::ToSocketAddrs, time::Duration};

const MAX_MANIFEST_BYTES: u64 = 256 * 1024;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct OfficialReleaseChecksum {
    pub distribution_id: String,
    pub release_label: String,
    pub manifest_url: String,
    pub file_name: String,
    pub sha256: String,
}

struct ChecksumSource {
    distribution_id: &'static str,
    release_label: &'static str,
    manifest_url: &'static str,
    allowed_host: &'static str,
}

// Fixed upstream endpoints only. No user-provided URLs or mirror redirects are accepted.
const SOURCES: &[ChecksumSource] = &[
    ChecksumSource {
        distribution_id: "debian",
        release_label: "Debian stable · amd64 · ISO CD",
        manifest_url: "https://cloudfront.debian.net/debian-cd/current/amd64/iso-cd/SHA256SUMS",
        allowed_host: "cloudfront.debian.net",
    },
    ChecksumSource {
        distribution_id: "arch",
        release_label: "Arch Linux current · x86_64",
        manifest_url: "https://archlinux.org/iso/latest/sha256sums.txt",
        allowed_host: "archlinux.org",
    },
    ChecksumSource {
        distribution_id: "ubuntu",
        release_label: "Ubuntu 24.04 LTS · official amd64 release manifest",
        manifest_url: "https://releases.ubuntu.com/24.04/SHA256SUMS",
        allowed_host: "releases.ubuntu.com",
    },
    ChecksumSource {
        distribution_id: "lubuntu",
        release_label: "Lubuntu 24.04 LTS · official amd64 release manifest",
        manifest_url: "https://cdimage.ubuntu.com/lubuntu/releases/24.04/release/SHA256SUMS",
        allowed_host: "cdimage.ubuntu.com",
    },
    ChecksumSource {
        distribution_id: "kubuntu",
        release_label: "Kubuntu 24.04 LTS · official amd64 release manifest",
        manifest_url: "https://cdimage.ubuntu.com/kubuntu/releases/24.04/release/SHA256SUMS",
        allowed_host: "cdimage.ubuntu.com",
    },
    ChecksumSource {
        distribution_id: "xubuntu",
        release_label: "Xubuntu 24.04 LTS · official amd64 release manifest",
        manifest_url: "https://cdimage.ubuntu.com/xubuntu/releases/24.04/release/SHA256SUMS",
        allowed_host: "cdimage.ubuntu.com",
    },
    ChecksumSource {
        distribution_id: "fedora",
        release_label: "Fedora Workstation 45 · x86_64",
        manifest_url: "https://dl.fedoraproject.org/pub/fedora/linux/releases/45/Workstation/x86_64/iso/Fedora-Workstation-45-1.4-x86_64-CHECKSUM",
        allowed_host: "dl.fedoraproject.org",
    },
    ChecksumSource {
        distribution_id: "opensuse",
        release_label: "openSUSE Tumbleweed · DVD x86_64 current",
        manifest_url: "https://download.opensuse.org/tumbleweed/iso/openSUSE-Tumbleweed-DVD-x86_64-Current.iso.sha256",
        allowed_host: "download.opensuse.org",
    },
    ChecksumSource {
        distribution_id: "antix",
        release_label: "antiX 26.1 · hashes publicados en la página oficial",
        manifest_url: "https://antixlinux.com/download/",
        allowed_host: "antixlinux.com",
    },
];

fn safe_iso_name(value: &str) -> bool {
    !value.is_empty()
        && value.to_ascii_lowercase().ends_with(".iso")
        && value
            == std::path::Path::new(value)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
        && !value.contains("..")
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "._-".contains(character))
}

pub fn supported_release_sources() -> Vec<(String, String)> {
    let mut sources = SOURCES
        .iter()
        .map(|source| {
            (
                source.distribution_id.to_owned(),
                source.release_label.to_owned(),
            )
        })
        .collect::<Vec<_>>();
    sources.push((
        "rocky-linux".to_owned(),
        "Rocky Linux 9/10 · CHECKSUM oficial x86_64".to_owned(),
    ));
    sources.push((
        "almalinux".to_owned(),
        "AlmaLinux 9/10 · CHECKSUM oficial x86_64".to_owned(),
    ));
    sources.push((
        "oracle-linux".to_owned(),
        "Oracle Linux 9/10 · checksum oficial x86_64".to_owned(),
    ));
    sources.push((
        "centos-stream".to_owned(),
        "CentOS Stream 9/10 · SHA256SUM oficial x86_64".to_owned(),
    ));
    sources
}

fn rocky_checksum_url_for_iso(file_name: &str) -> Result<(String, String), String> {
    if !safe_iso_name(file_name) || !file_name.starts_with("Rocky-") {
        return Err("Indica el nombre exacto de una ISO Rocky Linux.".into());
    }
    let stem = file_name.strip_suffix(".iso").unwrap_or_default();
    let parts = stem.split('-').collect::<Vec<_>>();
    if parts.len() < 4 || parts[0] != "Rocky" {
        return Err("El nombre de ISO Rocky Linux no tiene el formato oficial esperado.".into());
    }
    let release = parts[1];
    if release.is_empty()
        || !release.bytes().all(|byte| byte.is_ascii_digit() || byte == b'.')
        || release.starts_with('.')
        || release.ends_with('.')
        || release.contains("..")
    {
        return Err("La versión de Rocky Linux no es válida.".into());
    }
    let major = release.split('.').next().unwrap_or_default();
    if !matches!(major, "9" | "10") {
        return Err("El adaptador Rocky Linux cubre únicamente las ramas 9 y 10.".into());
    }
    let architecture_index = match parts.as_slice() {
        [_, _, "x86_64", _] => 2,
        [_, _, "latest", "x86_64", _] => 3,
        _ => return Err("El adaptador Rocky Linux cubre únicamente nombres x86_64 con formato oficial.".into()),
    };
    if architecture_index + 1 != parts.len() - 1
        || !matches!(parts.last().copied(), Some("boot" | "dvd" | "minimal"))
    {
        return Err("El tipo de ISO Rocky Linux no está admitido por este adaptador.".into());
    }
    Ok((
        format!("https://download.rockylinux.org/pub/rocky/{major}/isos/x86_64/CHECKSUM"),
        "download.rockylinux.org".to_owned(),
    ))
}

fn lookup_pinned_checksum_manifest(
    distribution_id: &str,
    release_label: &str,
    file_name: &str,
    manifest_url: &str,
    host: &str,
) -> Result<OfficialReleaseChecksum, String> {
    let url = reqwest::Url::parse(manifest_url)
        .map_err(|error| format!("URL del checksum oficial inválida: {error}"))?;
    if url.scheme() != "https" || url.host_str() != Some(host) {
        return Err("La fuente de checksum no supera la validación HTTPS del editor.".into());
    }
    let port = url.port_or_known_default()
        .ok_or_else(|| "No se pudo determinar el puerto HTTPS de la fuente.".to_string())?;
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| format!("No se pudo resolver {host}: {error}"))?
        .collect::<Vec<_>>();
    if addresses.is_empty()
        || addresses.iter().any(|address| !crate::download::is_safe_ip(address.ip()))
    {
        return Err(format!("La fuente oficial {host} resolvió a una dirección no pública."));
    }
    let client = reqwest::blocking::Client::builder()
        .resolve_to_addrs(host, &addresses)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(15))
        .user_agent("VoidBoot-OfficialChecksum/0.1")
        .build()
        .map_err(|error| format!("No se pudo preparar la consulta a {host}: {error}"))?;
    let response = client.get(manifest_url).send()
        .map_err(|error| format!("No se pudo consultar el checksum oficial: {error}"))?;
    if !response.status().is_success() || response.url().as_str() != manifest_url {
        return Err(format!("{host} respondió con {} o intentó redirigir; no se siguieron redirecciones.", response.status()));
    }
    if response.content_length().is_some_and(|length| length > MAX_MANIFEST_BYTES) {
        return Err("El manifiesto oficial supera el límite de tamaño permitido.".into());
    }
    let mut manifest = String::new();
    response.take(MAX_MANIFEST_BYTES + 1).read_to_string(&mut manifest)
        .map_err(|error| format!("No se pudo leer el manifiesto oficial: {error}"))?;
    if manifest.len() as u64 > MAX_MANIFEST_BYTES {
        return Err("El manifiesto oficial supera el límite de tamaño permitido.".into());
    }
    let sha256 = crate::parse_manifest_sha256(&manifest, file_name)
        .map_err(|error| format!("No se encontró un SHA-256 único para {file_name}: {error}"))?;
    Ok(OfficialReleaseChecksum {
        distribution_id: distribution_id.to_owned(),
        release_label: release_label.to_owned(),
        manifest_url: manifest_url.to_owned(),
        file_name: file_name.to_owned(),
        sha256,
    })
}

fn lookup_rocky_release_checksum(file_name: &str) -> Result<OfficialReleaseChecksum, String> {
    let (manifest_url, host) = rocky_checksum_url_for_iso(file_name)?;
    lookup_pinned_checksum_manifest(
        "rocky-linux",
        "Rocky Linux · CHECKSUM oficial x86_64",
        file_name,
        &manifest_url,
        &host,
    )
}

fn almalinux_checksum_url_for_iso(file_name: &str) -> Result<(String, String), String> {
    if !safe_iso_name(file_name) || !file_name.starts_with("AlmaLinux-") {
        return Err("Indica el nombre exacto de una ISO AlmaLinux.".into());
    }
    let stem = file_name.strip_suffix(".iso").unwrap_or_default();
    let parts = stem.split('-').collect::<Vec<_>>();
    if parts.len() < 4 || parts[0] != "AlmaLinux" {
        return Err("El nombre de ISO AlmaLinux no tiene el formato oficial esperado.".into());
    }
    let release = parts[1];
    if release.is_empty()
        || !release.bytes().all(|byte| byte.is_ascii_digit() || byte == b'.')
        || release.starts_with('.')
        || release.ends_with('.')
        || release.contains("..")
    {
        return Err("La versión de AlmaLinux no es válida.".into());
    }
    let major = release.split('.').next().unwrap_or_default();
    if !matches!(major, "9" | "10") {
        return Err("El adaptador AlmaLinux cubre únicamente las ramas 9 y 10.".into());
    }
    let architecture_index = match parts.as_slice() {
        [_, _, "x86_64", _] => 2,
        [_, _, "latest", "x86_64", _] => 3,
        _ => return Err("El adaptador AlmaLinux cubre únicamente nombres x86_64 con formato oficial.".into()),
    };
    if architecture_index + 1 != parts.len() - 1
        || !matches!(parts.last().copied(), Some("boot" | "dvd" | "minimal"))
    {
        return Err("El tipo de ISO AlmaLinux no está admitido por este adaptador.".into());
    }
    Ok((
        format!("https://repo.almalinux.org/almalinux/{major}/isos/x86_64/CHECKSUM"),
        "repo.almalinux.org".to_owned(),
    ))
}

fn lookup_almalinux_release_checksum(file_name: &str) -> Result<OfficialReleaseChecksum, String> {
    let (manifest_url, host) = almalinux_checksum_url_for_iso(file_name)?;
    lookup_pinned_checksum_manifest(
        "almalinux",
        "AlmaLinux · CHECKSUM oficial x86_64",
        file_name,
        &manifest_url,
        &host,
    )
}

fn oracle_linux_checksum_url_for_iso(file_name: &str) -> Result<(String, String), String> {
    if !safe_iso_name(file_name) || !file_name.starts_with("OracleLinux-") {
        return Err("Indica el nombre exacto de una ISO Oracle Linux.".into());
    }
    let stem = file_name.strip_suffix(".iso").unwrap_or_default();
    let parts = stem.split('-').collect::<Vec<_>>();
    if parts.len() != 5 || parts[0] != "OracleLinux" {
        return Err("El nombre de ISO Oracle Linux no tiene el formato oficial esperado.".into());
    }
    let release = parts[1];
    let major = release.strip_prefix('R').unwrap_or_default();
    if !matches!(major, "9" | "10") {
        return Err("El adaptador Oracle Linux cubre únicamente las ramas 9 y 10.".into());
    }
    let update = parts[2];
    let update_number = update.strip_prefix('U').unwrap_or_default();
    if update_number.is_empty() || !update_number.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("La actualización Oracle Linux debe usar el formato U<n>.".into());
    }
    if parts[3] != "x86_64" || !matches!(parts[4], "dvd" | "boot" | "boot-uek") {
        return Err("El adaptador Oracle Linux cubre ISO x86_64 DVD/boot/boot-uek.".into());
    }
    let checksum_name = format!("OracleLinux-{release}-{update}-Server-x86_64.checksum");
    Ok((
        format!("https://linux.oracle.com/security/gpg/checksum/{checksum_name}"),
        "linux.oracle.com".to_owned(),
    ))
}

fn lookup_oracle_linux_release_checksum(file_name: &str) -> Result<OfficialReleaseChecksum, String> {
    let (manifest_url, host) = oracle_linux_checksum_url_for_iso(file_name)?;
    lookup_pinned_checksum_manifest(
        "oracle-linux",
        "Oracle Linux · checksum oficial x86_64 (firma OpenPGP no verificada)",
        file_name,
        &manifest_url,
        &host,
    )
}

fn centos_stream_checksum_url_for_iso(file_name: &str) -> Result<(String, String), String> {
    if !safe_iso_name(file_name) || !file_name.starts_with("CentOS-Stream-") {
        return Err("Indica el nombre exacto de una ISO CentOS Stream.".into());
    }
    let parts = file_name.strip_suffix(".iso").unwrap_or_default().split('-').collect::<Vec<_>>();
    if parts.len() != 6 || parts[0] != "CentOS" || parts[1] != "Stream" {
        return Err("El nombre de ISO CentOS Stream no tiene el formato oficial esperado.".into());
    }
    let major = parts[2];
    if !matches!(major, "9" | "10") {
        return Err("El adaptador CentOS Stream cubre únicamente las ramas 9 y 10.".into());
    }
    let compose = parts[3];
    if compose != "latest" {
        let mut pieces = compose.split('.');
        let date = pieces.next().unwrap_or_default();
        let respin = pieces.next().unwrap_or_default();
        if pieces.next().is_some()
            || date.len() != 8
            || !date.bytes().all(|byte| byte.is_ascii_digit())
            || respin.is_empty()
            || !respin.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err("La composición CentOS Stream debe ser latest o YYYYMMDD.N.".into());
        }
    }
    if parts[4] != "x86_64" || !matches!(parts[5], "boot" | "dvd1") {
        return Err("El adaptador CentOS Stream cubre ISO x86_64 boot/dvd1.".into());
    }
    let stream = if major == "9" { "9-stream" } else { "10-stream" };
    let url = format!(
        "https://mirror.stream.centos.org/{stream}/BaseOS/x86_64/iso/{file_name}.SHA256SUM"
    );
    Ok((url, "mirror.stream.centos.org".to_owned()))
}

fn lookup_centos_stream_release_checksum(file_name: &str) -> Result<OfficialReleaseChecksum, String> {
    let (manifest_url, host) = centos_stream_checksum_url_for_iso(file_name)?;
    lookup_pinned_checksum_manifest(
        "centos-stream",
        "CentOS Stream · SHA256SUM oficial x86_64",
        file_name,
        &manifest_url,
        &host,
    )
}

fn parse_antix_sha256_html(html: &str, file_name: &str) -> Result<String, String> {
    if !safe_iso_name(file_name) || !file_name.starts_with("antiX-") {
        return Err("Indica el nombre exacto de una ISO antiX.".into());
    }

    // Strip markup while preserving token boundaries between adjacent HTML elements.
    let mut plain = String::with_capacity(html.len());
    let mut in_tag = false;
    for character in html.chars() {
        match character {
            '<' => {
                in_tag = true;
                plain.push(' ');
            }
            '>' => {
                in_tag = false;
                plain.push(' ');
            }
            _ if !in_tag => plain.push(character),
            _ => {}
        }
    }
    let plain = plain
        .replace("&nbsp;", " ")
        .replace("&#160;", " ")
        .replace("&amp;", "&");
    let tokens = plain.split_whitespace().collect::<Vec<_>>();
    let mut matches = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if *token != file_name || index == 0 {
            continue;
        }
        let digest = tokens[index - 1];
        if digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            matches.push(digest.to_ascii_lowercase());
        }
    }
    matches.sort();
    matches.dedup();
    match matches.as_slice() {
        [digest] => Ok(digest.clone()),
        [] => Err(format!("La página oficial antiX no publicó un SHA-256 para {file_name}.")),
        _ => Err(format!("La página oficial antiX publicó hashes SHA-256 contradictorios para {file_name}.")),
    }
}

pub fn lookup_official_release_checksum(
    distribution_id: &str,
    file_name: &str,
) -> Result<OfficialReleaseChecksum, String> {
    if !safe_iso_name(file_name) {
        return Err(
            "Indica el nombre exacto de una ISO, sin rutas ni caracteres especiales.".into(),
        );
    }

    if distribution_id == "rocky-linux" {
        return lookup_rocky_release_checksum(file_name);
    }

    if distribution_id == "almalinux" {
        return lookup_almalinux_release_checksum(file_name);
    }

    if distribution_id == "oracle-linux" {
        return lookup_oracle_linux_release_checksum(file_name);
    }

    if distribution_id == "centos-stream" {
        return lookup_centos_stream_release_checksum(file_name);
    }

    if distribution_id == "antix" {
        const PAGE_URL: &str = "https://antixlinux.com/download/";
        let html = fetch_official_mirror_page(
            PAGE_URL,
            "antixlinux.com",
            "VoidBoot-antiXChecksum/0.1",
        )?;
        let sha256 = parse_antix_sha256_html(&html, file_name)?;
        return Ok(OfficialReleaseChecksum {
            distribution_id: "antix".to_owned(),
            release_label: "antiX · SHA-256 publicado en la página oficial".to_owned(),
            manifest_url: PAGE_URL.to_owned(),
            file_name: file_name.to_owned(),
            sha256,
        });
    }

    let source = SOURCES
        .iter()
        .find(|source| source.distribution_id == distribution_id)
        .ok_or_else(|| {
            "Esta distribución todavía no tiene un adaptador oficial de checksums.".to_string()
        })?;

    let url = reqwest::Url::parse(source.manifest_url)
        .map_err(|error| format!("URL del adaptador inválida: {error}"))?;
    if url.scheme() != "https" || url.host_str() != Some(source.allowed_host) {
        return Err("La fuente oficial no supera la validación HTTPS del catálogo.".into());
    }

    let port = url
        .port_or_known_default()
        .ok_or_else(|| "No se pudo determinar el puerto HTTPS.".to_string())?;
    let addresses = (source.allowed_host, port)
        .to_socket_addrs()
        .map_err(|error| format!("No se pudo resolver la fuente oficial: {error}"))?
        .collect::<Vec<_>>();
    if addresses.is_empty()
        || addresses
            .iter()
            .any(|address| !crate::download::is_safe_ip(address.ip()))
    {
        return Err(
            "La fuente oficial resolvió a una dirección no pública; se canceló la consulta.".into(),
        );
    }

    let client = reqwest::blocking::Client::builder()
        .resolve_to_addrs(source.allowed_host, &addresses)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(15))
        .user_agent("VoidBoot-ReleaseMetadata/0.1")
        .build()
        .map_err(|error| format!("No se pudo preparar la consulta HTTPS: {error}"))?;

    let response = client
        .get(source.manifest_url)
        .send()
        .map_err(|error| format!("No se pudo consultar el manifiesto oficial: {error}"))?;
    if !response.status().is_success() || response.url().as_str() != source.manifest_url {
        return Err(format!(
            "La fuente oficial respondió con {} o intentó redirigir; no se siguieron redirecciones.",
            response.status()
        ));
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_MANIFEST_BYTES)
    {
        return Err("El manifiesto oficial supera el límite de tamaño permitido.".into());
    }

    let mut manifest = String::new();
    response
        .take(MAX_MANIFEST_BYTES + 1)
        .read_to_string(&mut manifest)
        .map_err(|error| format!("No se pudo leer el manifiesto oficial: {error}"))?;
    if manifest.len() as u64 > MAX_MANIFEST_BYTES {
        return Err("El manifiesto oficial supera el límite de tamaño permitido.".into());
    }

    let sha256 = crate::parse_manifest_sha256(&manifest, file_name)
        .map_err(|error| format!("No se encontró un SHA-256 único para {file_name}: {error}"))?;

    Ok(OfficialReleaseChecksum {
        distribution_id: source.distribution_id.to_owned(),
        release_label: source.release_label.to_owned(),
        manifest_url: source.manifest_url.to_owned(),
        file_name: file_name.to_owned(),
        sha256,
    })
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OfficialMirror {
    pub base_url: String,
    pub country: String,
    pub delay_seconds: u64,
    pub score: f64,
}

const ARCH_MIRROR_STATUS_URL: &str = "https://archlinux.org/mirrors/status/json/";
const MAX_MIRROR_STATUS_BYTES: u64 = 2 * 1024 * 1024;

/// Fetch active, recently synchronized HTTPS mirrors from Arch Linux's own status API.
pub fn list_arch_official_mirrors() -> Result<Vec<OfficialMirror>, String> {
    let url = reqwest::Url::parse(ARCH_MIRROR_STATUS_URL)
        .map_err(|error| format!("URL del estado de mirrors inválida: {error}"))?;
    let host = url
        .host_str()
        .ok_or_else(|| "La API oficial no tiene host.".to_string())?;
    let port = url
        .port_or_known_default()
        .ok_or_else(|| "Puerto HTTPS inválido.".to_string())?;
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| format!("No se pudo resolver la API oficial de Arch: {error}"))?
        .collect::<Vec<_>>();
    if addresses.is_empty()
        || addresses
            .iter()
            .any(|address| !crate::download::is_safe_ip(address.ip()))
    {
        return Err("La API oficial de Arch resolvió a una dirección no pública.".into());
    }

    let client = reqwest::blocking::Client::builder()
        .resolve_to_addrs(host, &addresses)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(20))
        .user_agent("VoidBoot-MirrorSelector/0.1")
        .build()
        .map_err(|error| format!("No se pudo preparar la consulta de mirrors: {error}"))?;
    let response = client
        .get(ARCH_MIRROR_STATUS_URL)
        .send()
        .map_err(|error| format!("No se pudo consultar el estado oficial de mirrors: {error}"))?;
    if !response.status().is_success() || response.url().as_str() != ARCH_MIRROR_STATUS_URL {
        return Err(format!(
            "La API de mirrors respondió con {}; no se siguieron redirecciones.",
            response.status()
        ));
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_MIRROR_STATUS_BYTES)
    {
        return Err("La lista oficial de mirrors supera el límite permitido.".into());
    }
    let mut body = Vec::new();
    response
        .take(MAX_MIRROR_STATUS_BYTES + 1)
        .read_to_end(&mut body)
        .map_err(|error| format!("No se pudo leer la lista oficial de mirrors: {error}"))?;
    if body.len() as u64 > MAX_MIRROR_STATUS_BYTES {
        return Err("La lista oficial de mirrors supera el límite permitido.".into());
    }
    let document: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|error| format!("La API oficial devolvió JSON inválido: {error}"))?;
    parse_arch_mirror_status(&document)
}

fn parse_arch_mirror_status(document: &serde_json::Value) -> Result<Vec<OfficialMirror>, String> {
    let entries = document
        .get("urls")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "La API oficial no contiene la lista de mirrors esperada.".to_string())?;

    let mut mirrors = Vec::new();
    for entry in entries {
        if entry.get("active").and_then(serde_json::Value::as_bool) != Some(true)
            || entry.get("protocol").and_then(serde_json::Value::as_str) != Some("https")
        {
            continue;
        }
        let completion = entry
            .get("completion_pct")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0);
        let delay = entry.get("delay").and_then(serde_json::Value::as_f64);
        let Some(delay_seconds) = delay
            .filter(|value| value.is_finite() && *value >= 0.0)
            .map(|value| value.round() as u64)
        else {
            continue;
        };
        let score = entry
            .get("score")
            .and_then(serde_json::Value::as_f64)
            .filter(|value| value.is_finite())
            .unwrap_or(delay_seconds as f64);
        if completion < 99.0 || delay_seconds > 6 * 60 * 60 {
            continue;
        }
        let Some(raw_url) = entry.get("url").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let Ok(mirror_url) = reqwest::Url::parse(raw_url) else {
            continue;
        };
        if mirror_url.scheme() != "https"
            || mirror_url.host_str().is_none()
            || !mirror_url.username().is_empty()
            || mirror_url.password().is_some()
            || mirror_url.query().is_some()
            || mirror_url.fragment().is_some()
            || !mirror_url.path().ends_with('/')
        {
            continue;
        }
        let country = entry
            .get("country")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("Ubicación no informada")
            .to_owned();
        let base_url = mirror_url.to_string();
        if mirrors
            .iter()
            .any(|mirror: &OfficialMirror| mirror.base_url == base_url)
        {
            continue;
        }
        mirrors.push(OfficialMirror {
            base_url,
            country,
            delay_seconds,
            score,
        });
    }

    mirrors.sort_by(|left, right| {
        let left_local = left.country.eq_ignore_ascii_case("Chile");
        let right_local = right.country.eq_ignore_ascii_case("Chile");
        right_local
            .cmp(&left_local)
            .then_with(|| left.score.total_cmp(&right.score))
    });
    mirrors.truncate(20);
    if mirrors.is_empty() {
        return Err(
            "La API oficial no informó mirrors HTTPS activos y suficientemente sincronizados."
                .into(),
        );
    }
    Ok(mirrors)
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct KaliDownloadCandidate {
    pub url: String,
    pub priority: u32,
    pub geo: String,
}

/// Parse only HTTPS candidates explicitly advertised by Kali's official image redirector.
fn parse_kali_redirect_candidates(
    location: Option<&str>,
    link_headers: &[String],
    artifact_path: &str,
) -> Result<Vec<KaliDownloadCandidate>, String> {
    fn validate(raw: &str, artifact_path: &str) -> Option<String> {
        let url = reqwest::Url::parse(raw).ok()?;
        if url.scheme() != "https"
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || !url.path().ends_with(artifact_path)
        {
            return None;
        }
        Some(url.to_string())
    }

    let mut candidates = Vec::new();
    if let Some(raw) = location {
        if let Some(url) = validate(raw, artifact_path) {
            candidates.push(KaliDownloadCandidate {
                url,
                priority: 0,
                geo: "redirector".to_owned(),
            });
        }
    }

    for header in link_headers {
        for item in header.split(',') {
            let item = item.trim();
            let item = item.strip_prefix('<').unwrap_or(item);
            let Some((target, parameters)) = item.split_once('>') else {
                continue;
            };
            let target = target.trim();
            let parameters = parameters.to_ascii_lowercase();
            if !parameters.contains("rel=duplicate") {
                continue;
            }
            let Some(url) = validate(target, artifact_path) else {
                continue;
            };
            let priority = parameters
                .split(';')
                .find_map(|part| part.trim().strip_prefix("pri="))
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or(1000);
            let geo = parameters
                .split(';')
                .find_map(|part| part.trim().strip_prefix("geo="))
                .unwrap_or("global")
                .trim_matches('"')
                .to_owned();
            candidates.push(KaliDownloadCandidate { url, priority, geo });
        }
    }

    candidates.sort_by_key(|left| left.priority);
    candidates.dedup_by(|left, right| left.url == right.url);
    if candidates.is_empty() {
        return Err(
            "El redirector oficial de Kali no entregó candidatos HTTPS válidos para esta ISO."
                .into(),
        );
    }
    Ok(candidates)
}

/// Ask Kali's official cdimage redirector for the current best mirror and its advertised fallbacks.
/// artifact_path must be a relative path such as kali-2026.3/kali-linux-2026.3-installer-amd64.iso.
pub fn list_kali_official_download_candidates(
    artifact_path: &str,
) -> Result<Vec<KaliDownloadCandidate>, String> {
    if artifact_path.is_empty()
        || artifact_path.starts_with('/')
        || artifact_path.contains("..")
        || artifact_path.contains('\\')
        || !artifact_path
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"/._-".contains(&byte))
        || !artifact_path.to_ascii_lowercase().ends_with(".iso")
    {
        return Err("Indica una ruta relativa válida de ISO de Kali, sin rutas absolutas ni segmentos '..'.".into());
    }

    const REDIRECTOR: &str = "https://cdimage.kali.org/";
    let url = format!("{REDIRECTOR}{artifact_path}");
    let host = "cdimage.kali.org";
    let addresses = (host, 443)
        .to_socket_addrs()
        .map_err(|error| format!("No se pudo resolver el redirector oficial de Kali: {error}"))?
        .collect::<Vec<_>>();
    if addresses.is_empty()
        || addresses
            .iter()
            .any(|address| !crate::download::is_safe_ip(address.ip()))
    {
        return Err("El redirector oficial de Kali resolvió a una dirección no pública.".into());
    }

    let client = reqwest::blocking::Client::builder()
        .resolve_to_addrs(host, &addresses)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(15))
        .user_agent("VoidBoot-KaliMirrorResolver/0.1")
        .build()
        .map_err(|error| format!("No se pudo preparar la consulta al redirector: {error}"))?;
    let response = client
        .get(&url)
        .header(reqwest::header::RANGE, "bytes=0-0")
        .send()
        .map_err(|error| format!("No se pudo consultar el redirector oficial de Kali: {error}"))?;
    if !response.status().is_redirection() || response.url().as_str() != url {
        return Err(format!(
            "El redirector oficial de Kali respondió con {}; no se siguieron redirecciones.",
            response.status()
        ));
    }

    let location = response
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|value| value.to_str().ok());
    let links = response
        .headers()
        .get_all(reqwest::header::LINK)
        .iter()
        .filter_map(|value| value.to_str().ok().map(str::to_owned))
        .collect::<Vec<_>>();
    parse_kali_redirect_candidates(location, &links, artifact_path)
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct OfficialPathDownloadCandidate {
    pub url: String,
    pub provider: String,
    pub region: String,
}

/// Resolve Fedora's official download redirector for one exact ISO path.
/// MirrorManager chooses the mirror; only HTTPS redirects for the exact artifact are accepted.
pub fn list_fedora_official_download_candidates(
    release: &str,
    edition: &str,
    architecture: &str,
    file_name: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    fn safe_segment(value: &str) -> bool {
        !value.is_empty()
            && value.len() <= 80
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
            && !value.contains("..")
    }

    if !safe_segment(release)
        || !safe_segment(edition)
        || !matches!(architecture, "x86_64" | "aarch64")
        || !safe_segment(file_name)
        || !file_name.to_ascii_lowercase().ends_with(".iso")
    {
        return Err("Indica versión, edición, arquitectura y nombre de ISO Fedora válidos.".into());
    }

    let artifact_path = format!("{release}/{edition}/{architecture}/iso/{file_name}");
    let redirector = "download.fedoraproject.org";
    let url = format!("https://{redirector}/pub/fedora/linux/releases/{artifact_path}");
    let addresses = (redirector, 443)
        .to_socket_addrs()
        .map_err(|error| format!("No se pudo resolver el redirector oficial de Fedora: {error}"))?
        .collect::<Vec<_>>();
    if addresses.is_empty()
        || addresses
            .iter()
            .any(|address| !crate::download::is_safe_ip(address.ip()))
    {
        return Err("El redirector oficial de Fedora resolvió a una dirección no pública.".into());
    }

    let client = reqwest::blocking::Client::builder()
        .resolve_to_addrs(redirector, &addresses)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(15))
        .user_agent("VoidBoot-FedoraMirrorResolver/0.1")
        .build()
        .map_err(|error| format!("No se pudo preparar la consulta a Fedora: {error}"))?;
    let response = client
        .get(&url)
        .header(reqwest::header::RANGE, "bytes=0-0")
        .send()
        .map_err(|error| {
            format!("No se pudo consultar el redirector oficial de Fedora: {error}")
        })?;
    if !response.status().is_redirection() || response.url().as_str() != url {
        return Err(format!(
            "El redirector oficial de Fedora respondió con {}; no se siguieron redirecciones.",
            response.status()
        ));
    }

    let location = response
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| "Fedora no anunció un destino de mirror en la respuesta.".to_string())?;
    let candidate = reqwest::Url::parse(location)
        .map_err(|error| format!("Fedora anunció una URL de mirror inválida: {error}"))?;
    if candidate.scheme() != "https"
        || candidate.host_str().is_none()
        || !candidate.username().is_empty()
        || candidate.password().is_some()
        || candidate.query().is_some()
        || candidate.fragment().is_some()
        || !candidate.path().ends_with(&artifact_path)
    {
        return Err(
            "El destino anunciado por Fedora no es HTTPS o no coincide con la ISO exacta.".into(),
        );
    }

    Ok(vec![OfficialPathDownloadCandidate {
        url: candidate.to_string(),
        provider: "Fedora MirrorManager".to_owned(),
        region: "Mirror oficial asignado".to_owned(),
    }])
}

/// Resolve official Ubuntu and flavour ISO locations. This is a publisher endpoint,
/// not a claim that every image has a selectable mirror list.
pub fn list_ubuntu_official_download_candidates(
    distribution_id: &str,
    release: &str,
    file_name: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    fn safe_segment(value: &str) -> bool {
        !value.is_empty()
            && value.len() <= 80
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
            && !value.contains("..")
    }

    if !safe_segment(release)
        || !safe_segment(file_name)
        || !file_name.to_ascii_lowercase().ends_with(".iso")
    {
        return Err("Indica una versión y un nombre de ISO válidos.".into());
    }

    let url = match distribution_id {
        "ubuntu" => format!("https://releases.ubuntu.com/{release}/{file_name}"),
        "lubuntu" | "kubuntu" | "xubuntu" => {
            format!("https://cdimage.ubuntu.com/{distribution_id}/releases/{release}/release/{file_name}")
        }
        _ => {
            return Err(
                "Esta distribución no pertenece a los canales oficiales Ubuntu admitidos.".into(),
            )
        }
    };
    let parsed = reqwest::Url::parse(&url)
        .map_err(|error| format!("La URL oficial de Ubuntu es inválida: {error}"))?;
    let expected_host = if distribution_id == "ubuntu" {
        "releases.ubuntu.com"
    } else {
        "cdimage.ubuntu.com"
    };
    if parsed.scheme() != "https" || parsed.host_str() != Some(expected_host) {
        return Err("La URL no pertenece al servidor HTTPS oficial esperado de Ubuntu.".into());
    }

    Ok(vec![OfficialPathDownloadCandidate {
        url,
        provider: if distribution_id == "ubuntu" {
            "Ubuntu Releases".to_owned()
        } else {
            "Ubuntu CD Image".to_owned()
        },
        region: "Servidor oficial".to_owned(),
    }])
}

const MAX_OPENSUSE_MIRRORLIST_BYTES: u64 = 2 * 1024 * 1024;

const MAX_BSD_MIRROR_PAGE_BYTES: u64 = 2 * 1024 * 1024;

fn parse_publisher_mirror_directory(
    html: &str,
    page_url: &str,
    artifact_path: &str,
    root_markers: &[&str],
    provider: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    let page = reqwest::Url::parse(page_url)
        .map_err(|error| format!("Página oficial de mirrors inválida: {error}"))?;
    let mut candidates: Vec<OfficialPathDownloadCandidate> = Vec::new();
    let mut cursor = 0;

    while cursor < html.len() {
        let Some(relative_index) = html[cursor..].to_ascii_lowercase().find("href") else {
            break;
        };
        let mut position = cursor + relative_index + 4;
        while html
            .as_bytes()
            .get(position)
            .is_some_and(u8::is_ascii_whitespace)
        {
            position += 1;
        }
        if html.as_bytes().get(position) != Some(&b'=') {
            cursor = position;
            continue;
        }
        position += 1;
        while html
            .as_bytes()
            .get(position)
            .is_some_and(u8::is_ascii_whitespace)
        {
            position += 1;
        }
        let Some(&quote) = html.as_bytes().get(position) else {
            break;
        };
        if quote != b'"' && quote != b'\'' {
            cursor = position + 1;
            continue;
        }
        let value_start = position + 1;
        let Some(relative_end) = html[value_start..].find(quote as char) else {
            break;
        };
        let value_end = value_start + relative_end;
        let raw = html[value_start..value_end].replace("&amp;", "&");
        cursor = value_end + 1;

        let Ok(base) = page.join(&raw) else {
            continue;
        };
        let base_path = base.path().to_ascii_lowercase();
        if base.scheme() != "https"
            || base.host_str().is_none()
            || !base.username().is_empty()
            || base.password().is_some()
            || base.query().is_some()
            || base.fragment().is_some()
            || !base.path().ends_with('/')
            || !root_markers
                .iter()
                .any(|marker| base_path.ends_with(&marker.to_ascii_lowercase()))
        {
            continue;
        }

        let Ok(url) = base.join(artifact_path) else {
            continue;
        };
        if url.scheme() != "https" || !url.path().ends_with(artifact_path) {
            continue;
        }
        let candidate_url = url.to_string();
        if candidates.iter().any(|item| item.url == candidate_url) {
            continue;
        }
        candidates.push(OfficialPathDownloadCandidate {
            url: candidate_url,
            provider: provider.to_owned(),
            region: base.host_str().unwrap_or("mirror").to_owned(),
        });
        if candidates.len() == 30 {
            break;
        }
    }

    if candidates.is_empty() {
        return Err(format!(
            "{provider} no anunció mirrors HTTPS válidos para esa ruta."
        ));
    }
    Ok(candidates)
}

fn fetch_official_mirror_page(
    page_url: &str,
    host: &str,
    user_agent: &str,
) -> Result<String, String> {
    let addresses = (host, 443)
        .to_socket_addrs()
        .map_err(|error| format!("No se pudo resolver {host}: {error}"))?
        .collect::<Vec<_>>();
    if addresses.is_empty()
        || addresses
            .iter()
            .any(|address| !crate::download::is_safe_ip(address.ip()))
    {
        return Err(format!("{host} resolvió a una dirección no pública."));
    }
    let client = reqwest::blocking::Client::builder()
        .resolve_to_addrs(host, &addresses)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(15))
        .user_agent(user_agent)
        .build()
        .map_err(|error| format!("No se pudo preparar la consulta a {host}: {error}"))?;
    let response = client
        .get(page_url)
        .send()
        .map_err(|error| format!("No se pudo consultar {host}: {error}"))?;
    if !response.status().is_success() || response.url().as_str() != page_url {
        return Err(format!(
            "{host} respondió con {}; no se siguieron redirecciones.",
            response.status()
        ));
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_BSD_MIRROR_PAGE_BYTES)
    {
        return Err(format!(
            "La página de mirrors de {host} supera el límite permitido."
        ));
    }
    let mut html = String::new();
    response
        .take(MAX_BSD_MIRROR_PAGE_BYTES + 1)
        .read_to_string(&mut html)
        .map_err(|error| format!("No se pudo leer la página de mirrors de {host}: {error}"))?;
    if html.len() as u64 > MAX_BSD_MIRROR_PAGE_BYTES {
        return Err(format!(
            "La página de mirrors de {host} supera el límite permitido."
        ));
    }
    Ok(html)
}

/// Resolve Linux Mint ISO mirrors from the project's official mirror directory.
/// The mirror page links to ISO-specific roots; repository/package mirrors are excluded.
pub fn list_linux_mint_official_download_candidates(
    artifact_path: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    if !safe_relative_iso_path(artifact_path) {
        return Err("Indica una ruta relativa válida de ISO Linux Mint.".into());
    }
    let page_url = "https://www.linuxmint.com/mirrors.php";
    let html = fetch_official_mirror_page(
        page_url,
        "www.linuxmint.com",
        "VoidBoot-LinuxMintMirrorResolver/0.1",
    )?;
    parse_publisher_mirror_directory(
        &html,
        page_url,
        artifact_path,
        &["/iso/"],
        "Linux Mint official ISO mirror",
    )
}

/// Build a FreeBSD ISO URL on the project's official GeoDNS/CDN service.
pub fn list_freebsd_official_download_candidates(
    artifact_path: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    if !safe_relative_iso_path(artifact_path) {
        return Err("Indica una ruta relativa válida de ISO FreeBSD.".into());
    }
    let url = format!("https://download.freebsd.org/{artifact_path}");
    let parsed = reqwest::Url::parse(&url)
        .map_err(|error| format!("La URL oficial de FreeBSD es inválida: {error}"))?;
    if parsed.scheme() != "https" || parsed.host_str() != Some("download.freebsd.org") {
        return Err("La URL no pertenece al servidor HTTPS oficial de FreeBSD.".into());
    }
    Ok(vec![OfficialPathDownloadCandidate {
        url,
        provider: "FreeBSD GeoDNS / CDN".to_owned(),
        region: "Mirror oficial automático".to_owned(),
    }])
}

/// Resolve an exact OpenBSD ISO path against the project's published HTTPS mirror list.
pub fn list_openbsd_official_download_candidates(
    artifact_path: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    if !safe_relative_iso_path(artifact_path) {
        return Err("Indica una ruta relativa válida de ISO OpenBSD.".into());
    }
    let page_url = "https://www.openbsd.org/ftp.html";
    let html = fetch_official_mirror_page(
        page_url,
        "www.openbsd.org",
        "VoidBoot-OpenBSDMirrorResolver/0.1",
    )?;
    parse_publisher_mirror_directory(
        &html,
        page_url,
        artifact_path,
        &["/pub/openbsd/", "/openbsd/"],
        "OpenBSD official mirror",
    )
}

/// Resolve an exact NetBSD ISO path against the project's published HTTPS mirror list.
pub fn list_netbsd_official_download_candidates(
    artifact_path: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    if !safe_relative_iso_path(artifact_path) {
        return Err("Indica una ruta relativa válida de ISO NetBSD.".into());
    }
    let page_url = "https://www.netbsd.org/mirrors/";
    let html = fetch_official_mirror_page(
        page_url,
        "www.netbsd.org",
        "VoidBoot-NetBSDMirrorResolver/0.1",
    )?;
    parse_publisher_mirror_directory(
        &html,
        page_url,
        artifact_path,
        &["/pub/netbsd/", "/netbsd/"],
        "NetBSD official mirror",
    )
}

/// Resolve Rocky Linux ISO paths on the project's official download host.
pub fn list_rocky_official_download_candidates(
    artifact_path: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    if !safe_relative_iso_path(artifact_path) {
        return Err("Indica una ruta relativa válida de ISO Rocky Linux.".into());
    }
    if !artifact_path.starts_with("10/isos/") && !artifact_path.starts_with("9/isos/") {
        return Err("La ruta Rocky debe comenzar por 9/isos/ o 10/isos/.".into());
    }
    let url = format!("https://download.rockylinux.org/pub/rocky/{artifact_path}");
    let parsed = reqwest::Url::parse(&url)
        .map_err(|error| format!("La URL oficial de Rocky Linux es inválida: {error}"))?;
    if parsed.scheme() != "https"
        || parsed.host_str() != Some("download.rockylinux.org")
        || !parsed.path().ends_with(".iso")
    {
        return Err("La URL no pertenece al servidor HTTPS oficial de Rocky Linux.".into());
    }
    Ok(vec![OfficialPathDownloadCandidate {
        url,
        provider: "Rocky Linux official download".to_owned(),
        region: "CDN / servidor oficial".to_owned(),
    }])
}

/// Resolve AlmaLinux ISO paths on the Foundation's official CDN-backed master host.
pub fn list_almalinux_official_download_candidates(
    artifact_path: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    if !safe_relative_iso_path(artifact_path) {
        return Err("Indica una ruta relativa válida de ISO AlmaLinux.".into());
    }
    if !artifact_path.starts_with("10/isos/")
        && !artifact_path.starts_with("9/isos/")
    {
        return Err("La ruta AlmaLinux debe comenzar por 9/isos/ o 10/isos/.".into());
    }
    let url = format!("https://repo.almalinux.org/almalinux/{artifact_path}");
    let parsed = reqwest::Url::parse(&url)
        .map_err(|error| format!("La URL oficial de AlmaLinux es inválida: {error}"))?;
    if parsed.scheme() != "https"
        || parsed.host_str() != Some("repo.almalinux.org")
        || !parsed.path().ends_with(".iso")
    {
        return Err("La URL no pertenece al servidor HTTPS oficial de AlmaLinux.".into());
    }
    Ok(vec![OfficialPathDownloadCandidate {
        url,
        provider: "AlmaLinux official CDN".to_owned(),
        region: "Servidor maestro oficial".to_owned(),
    }])
}

/// Resolve CentOS Stream installer ISOs on the official CentOS mirror host.
pub fn list_centos_stream_official_download_candidates(
    artifact_path: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    if !safe_relative_iso_path(artifact_path) {
        return Err("Indica una ruta relativa válida de ISO CentOS Stream.".into());
    }
    let supported_tree = ["9-stream/", "10-stream/"]
        .iter()
        .any(|prefix| artifact_path.starts_with(prefix));
    let supported_layout = artifact_path.contains("/BaseOS/x86_64/iso/")
        || artifact_path.contains("/BaseOS/aarch64/iso/");
    if !supported_tree || !supported_layout {
        return Err("La ruta CentOS Stream debe usar 9-stream o 10-stream y un árbol BaseOS ISO x86_64/aarch64.".into());
    }
    let url = format!("https://mirror.stream.centos.org/{artifact_path}");
    let parsed = reqwest::Url::parse(&url)
        .map_err(|error| format!("La URL oficial de CentOS Stream es inválida: {error}"))?;
    if parsed.scheme() != "https"
        || parsed.host_str() != Some("mirror.stream.centos.org")
        || !parsed.path().ends_with(".iso")
    {
        return Err("La URL no pertenece al mirror HTTPS oficial de CentOS Stream.".into());
    }
    Ok(vec![OfficialPathDownloadCandidate {
        url,
        provider: "CentOS Stream official mirror".to_owned(),
        region: "Mirror oficial del proyecto".to_owned(),
    }])
}

/// Resolve Oracle Linux installer ISOs from Oracle's official ISO distribution host.
pub fn list_oracle_linux_official_download_candidates(
    artifact_path: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    if !safe_relative_iso_path(artifact_path) {
        return Err("Indica una ruta relativa válida de ISO Oracle Linux.".into());
    }
    let supported_version = artifact_path.starts_with("OL9/")
        || artifact_path.starts_with("OL10/");
    let supported_architecture = artifact_path.contains("/x86_64/")
        || artifact_path.contains("/aarch64/");
    if !supported_version || !supported_architecture {
        return Err("La ruta Oracle Linux debe usar OL9/OL10 y arquitectura x86_64/aarch64.".into());
    }
    let url = format!("https://yum.oracle.com/ISOS/OracleLinux/{artifact_path}");
    let parsed = reqwest::Url::parse(&url)
        .map_err(|error| format!("La URL oficial de Oracle Linux es inválida: {error}"))?;
    if parsed.scheme() != "https"
        || parsed.host_str() != Some("yum.oracle.com")
        || !parsed.path().ends_with(".iso")
    {
        return Err("La URL no pertenece al host HTTPS oficial de Oracle Linux.".into());
    }
    Ok(vec![OfficialPathDownloadCandidate {
        url,
        provider: "Oracle Linux official ISO host".to_owned(),
        region: "Servidor oficial de Oracle".to_owned(),
    }])
}

/// Build a Manjaro x86_64 edition ISO URL on the official download host.
pub fn list_manjaro_official_download_candidates(
    edition: &str,
    release: &str,
    file_name: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    fn safe_segment(value: &str) -> bool {
        !value.is_empty()
            && value.len() <= 100
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
            && !value.contains("..")
    }

    if !["gnome", "kde", "xfce"].contains(&edition) {
        return Err("La edición Manjaro debe ser gnome, kde o xfce.".into());
    }
    if !safe_segment(release)
        || !safe_segment(file_name)
        || !file_name.to_ascii_lowercase().ends_with(".iso")
    {
        return Err("Indica una versión y un nombre de ISO Manjaro válidos.".into());
    }
    let url = format!("https://download.manjaro.org/{edition}/{release}/{file_name}");
    let parsed = reqwest::Url::parse(&url)
        .map_err(|error| format!("La URL oficial de Manjaro es inválida: {error}"))?;
    if parsed.scheme() != "https" || parsed.host_str() != Some("download.manjaro.org") {
        return Err("La URL no pertenece al host HTTPS oficial de Manjaro.".into());
    }
    Ok(vec![OfficialPathDownloadCandidate {
        url,
        provider: "Manjaro official download".to_owned(),
        region: "Servidor oficial".to_owned(),
    }])
}

/// Resolve Zorin OS images from the Kernel.org mirror explicitly listed by Zorin.
pub fn list_zorin_official_download_candidates(
    release: &str,
    file_name: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    let safe_release = !release.is_empty()
        && release.len() <= 8
        && release.bytes().all(|byte| byte.is_ascii_digit() || byte == b'.')
        && !release.starts_with('.')
        && !release.ends_with('.')
        && !release.contains("..");
    let safe_file = !file_name.is_empty()
        && file_name.len() <= 160
        && file_name.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
        && !file_name.contains("..")
        && file_name.to_ascii_lowercase().ends_with(".iso")
        && file_name.starts_with(&format!("Zorin-OS-{release}"));
    if !safe_release || !safe_file {
        return Err("Indica una versión y un nombre de ISO Zorin OS válidos.".into());
    }

    // Kernel.org is listed in Zorin's official mirror directory.
    let url = format!("https://mirrors.edge.kernel.org/zorinos-isos/{release}/{file_name}");
    let parsed = reqwest::Url::parse(&url)
        .map_err(|error| format!("La URL oficial de Zorin es inválida: {error}"))?;
    if parsed.scheme() != "https"
        || parsed.host_str() != Some("mirrors.edge.kernel.org")
        || !parsed.path().starts_with("/zorinos-isos/")
    {
        return Err("La URL no pertenece al mirror HTTPS publicado por Zorin.".into());
    }
    Ok(vec![OfficialPathDownloadCandidate {
        url,
        provider: "Zorin OS official-listed Kernel.org mirror".to_owned(),
        region: "Kernel.org · mirror global".to_owned(),
    }])
}

/// Build Pop!_OS ISO URLs on System76's official image host.
pub fn list_popos_official_download_candidates(
    release: &str,
    architecture: &str,
    channel: &str,
    build: &str,
    file_name: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    let safe_release = !release.is_empty()
        && release.len() <= 8
        && release.bytes().all(|byte| byte.is_ascii_digit() || byte == b'.')
        && !release.starts_with('.')
        && !release.ends_with('.')
        && !release.contains("..");
    let safe_build = !build.is_empty()
        && build.len() <= 6
        && build.bytes().all(|byte| byte.is_ascii_digit());
    let expected_name = format!("pop-os_{release}_{architecture}_{channel}_{build}.iso");
    if !safe_release || !safe_build || architecture != "amd64"
        || !["generic", "nvidia"].contains(&channel) || file_name != expected_name {
        return Err("Indica una versión, build, canal y nombre de ISO Pop!_OS válidos (amd64: generic o nvidia).".into());
    }
    let url = format!("https://iso.pop-os.org/{release}/{architecture}/{channel}/{build}/{file_name}");
    let parsed = reqwest::Url::parse(&url).map_err(|error| format!("La URL oficial de Pop!_OS es inválida: {error}"))?;
    if parsed.scheme() != "https" || parsed.host_str() != Some("iso.pop-os.org") {
        return Err("La URL no pertenece al host HTTPS oficial de Pop!_OS.".into());
    }
    Ok(vec![OfficialPathDownloadCandidate {
        url, provider: "System76 official Pop!_OS ISO host".to_owned(),
        region: "CDN oficial de System76".to_owned(),
    }])
}

/// Parse direct elementary OS download links published on the official website.
fn parse_elementary_official_download_links(
    html: &str, page_url: &str, file_name: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    let page = reqwest::Url::parse(page_url)
        .map_err(|error| format!("Página oficial de elementary OS inválida: {error}"))?;
    let mut candidates = Vec::new();
    let mut cursor = 0;
    while cursor < html.len() {
        let Some(relative_index) = html[cursor..].to_ascii_lowercase().find("href") else { break; };
        let mut position = cursor + relative_index + 4;
        while html.as_bytes().get(position).is_some_and(u8::is_ascii_whitespace) { position += 1; }
        if html.as_bytes().get(position) != Some(&b'=') { cursor = position; continue; }
        position += 1;
        while html.as_bytes().get(position).is_some_and(u8::is_ascii_whitespace) { position += 1; }
        let Some(&quote) = html.as_bytes().get(position) else { break; };
        if quote != b'"' && quote != b'\'' { cursor = position + 1; continue; }
        let value_start = position + 1;
        let Some(relative_end) = html[value_start..].find(quote as char) else { break; };
        let value_end = value_start + relative_end;
        let raw = html[value_start..value_end].replace("&amp;", "&");
        cursor = value_end + 1;
        let Ok(url) = page.join(&raw) else { continue; };
        if url.scheme() != "https" || url.host_str() != Some("dl.elementaryos.org")
            || !url.username().is_empty() || url.password().is_some()
            || url.query().is_some() || url.fragment().is_some()
            || url.path().rsplit('/').next() != Some(file_name) { continue; }
        let candidate_url = url.to_string();
        if candidates.iter().any(|item: &OfficialPathDownloadCandidate| item.url == candidate_url) { continue; }
        candidates.push(OfficialPathDownloadCandidate {
            url: candidate_url, provider: "elementary OS official download gateway".to_owned(),
            region: "Descarga oficial dinámica".to_owned(),
        });
        if candidates.len() == 10 { break; }
    }
    if candidates.is_empty() {
        return Err("La página oficial de elementary OS no anunció un enlace directo para ese nombre exacto de ISO.".into());
    }
    Ok(candidates)
}

/// Resolve Alpine Linux installer ISOs from the official latest-stable CDN tree.
pub fn list_alpine_official_download_candidates(
    artifact_path: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    if !safe_relative_iso_path(artifact_path)
        || !artifact_path.starts_with("latest-stable/releases/")
    {
        return Err("La ruta Alpine debe estar dentro de latest-stable/releases/.".into());
    }
    let parts: Vec<&str> = artifact_path.split('/').collect();
    if parts.len() != 4 {
        return Err("La ruta Alpine debe incluir arquitectura y nombre de ISO.".into());
    }
    let architecture = parts[2];
    if !["x86", "x86_64", "aarch64", "armhf", "armv7", "ppc64le", "riscv64", "s390x", "loongarch64"].contains(&architecture)
        || !parts[3].starts_with("alpine-")
        || !parts[3].ends_with(".iso")
        || !parts[3].contains(&format!("-{architecture}."))
    {
        return Err("La ruta Alpine debe indicar una arquitectura oficial y un nombre alpine-*.iso coincidente.".into());
    }
    let url = format!("https://dl-cdn.alpinelinux.org/alpine/{artifact_path}");
    let parsed = reqwest::Url::parse(&url)
        .map_err(|error| format!("La URL oficial de Alpine Linux es inválida: {error}"))?;
    if parsed.scheme() != "https" || parsed.host_str() != Some("dl-cdn.alpinelinux.org") {
        return Err("La URL no pertenece al CDN HTTPS oficial de Alpine Linux.".into());
    }
    Ok(vec![OfficialPathDownloadCandidate {
        url,
        provider: "Alpine Linux official CDN".to_owned(),
        region: "CDN oficial dl-cdn.alpinelinux.org".to_owned(),
    }])
}

/// Resolve NixOS stable/unstable installer ISOs through the official channel aliases.
pub fn list_nixos_official_download_candidates(
    channel: &str,
    variant: &str,
    architecture: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    let valid_channel = channel == "nixos-unstable"
        || (channel.starts_with("nixos-")
            && channel.len() == 11
            && channel.as_bytes()[6..8].iter().all(u8::is_ascii_digit)
            && channel.as_bytes()[8] == b'.'
            && channel.as_bytes()[9..11].iter().all(u8::is_ascii_digit));
    if !valid_channel {
        return Err("El canal NixOS debe ser nixos-unstable o un canal estable como nixos-26.05.".into());
    }
    if !["graphical", "minimal"].contains(&variant) {
        return Err("La variante NixOS debe ser graphical o minimal.".into());
    }
    if !["x86_64", "aarch64"].contains(&architecture) {
        return Err("La arquitectura NixOS debe ser x86_64 o aarch64.".into());
    }
    let file_name = format!("latest-nixos-{variant}-{architecture}-linux.iso");
    let url = format!("https://channels.nixos.org/{channel}/{file_name}");
    let parsed = reqwest::Url::parse(&url)
        .map_err(|error| format!("La URL oficial de NixOS es inválida: {error}"))?;
    if parsed.scheme() != "https" || parsed.host_str() != Some("channels.nixos.org") {
        return Err("La URL no pertenece al host HTTPS oficial de canales NixOS.".into());
    }
    Ok(vec![OfficialPathDownloadCandidate {
        url,
        provider: "NixOS official channel".to_owned(),
        region: "Alias oficial al ISO más reciente del canal".to_owned(),
    }])
}

/// Resolve Void Linux live installer ISOs from the official current image tree.
pub fn list_void_linux_official_download_candidates(
    artifact_path: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    if !safe_relative_iso_path(artifact_path) || !artifact_path.starts_with("live/current/") {
        return Err("La ruta Void Linux debe estar dentro de live/current/.".into());
    }
    let file_name = artifact_path.rsplit('/').next().unwrap_or_default();
    if !safe_iso_name(file_name)
        || !file_name.starts_with("void-live-")
        || !["x86_64", "i686", "aarch64"].iter().any(|arch| file_name.contains(&format!("-{arch}-")))
        || !(file_name.ends_with("-base.iso") || file_name.ends_with("-xfce.iso"))
    {
        return Err("La ruta debe señalar una ISO live oficial Void (base o xfce) con arquitectura admitida.".into());
    }
    let url = format!("https://repo-default.voidlinux.org/{artifact_path}");
    let parsed = reqwest::Url::parse(&url)
        .map_err(|error| format!("La URL oficial de Void Linux es inválida: {error}"))?;
    if parsed.scheme() != "https"
        || parsed.host_str() != Some("repo-default.voidlinux.org")
        || !parsed.path().ends_with(file_name)
    {
        return Err("La URL no pertenece al repositorio HTTPS oficial de Void Linux.".into());
    }
    Ok(vec![OfficialPathDownloadCandidate {
        url,
        provider: "Void Linux official image repository".to_owned(),
        region: "Árbol oficial live/current; endpoint único, no selector de mirrors".to_owned(),
    }])
}

/// Parse MX Linux's publisher-maintained mirror status page for exact ISO paths.
pub fn parse_mx_official_iso_mirrors(
    html: &str,
    artifact_path: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    if !safe_relative_iso_path(artifact_path)
        || !(artifact_path.starts_with("MX/Final/") || artifact_path.starts_with("MX/Snapshots/"))
        || !artifact_path.rsplit('/').next().is_some_and(|name| {
            name.starts_with("MX-") && name.to_ascii_lowercase().ends_with(".iso")
        })
    {
        return Err("La ruta MX debe señalar una ISO dentro de MX/Final/ o MX/Snapshots/.".into());
    }
    parse_publisher_mirror_directory(
        html,
        "https://rsync-mxlinux.org/mirmon/index.html",
        artifact_path,
        &["/iso/"],
        "MX Linux official ISO mirror",
    )
}

/// Resolve MX Linux ISO mirrors from the publisher-maintained live mirror status page.
pub fn list_mx_official_download_candidates(
    artifact_path: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    const PAGE_URL: &str = "https://rsync-mxlinux.org/mirmon/index.html";
    let html = fetch_official_mirror_page(
        PAGE_URL,
        "rsync-mxlinux.org",
        "VoidBoot-MXLinuxMirrorResolver/0.1",
    )?;
    parse_mx_official_iso_mirrors(&html, artifact_path)
}

/// Parse Mageia's official mirror database and build exact HTTPS ISO candidates.
pub fn parse_mageia_official_mirrors(
    html: &str,
    artifact_path: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    if !safe_relative_iso_path(artifact_path)
        || !artifact_path.starts_with("iso/")
        || !artifact_path.rsplit('/').next().is_some_and(|name| {
            name.to_ascii_lowercase().ends_with(".iso")
                && name.starts_with("Mageia-")
        })
    {
        return Err("La ruta Mageia debe señalar una ISO con nombre Mageia- dentro de iso/.".into());
    }
    let page = reqwest::Url::parse("https://mirrors.mageia.org/")
        .expect("static Mageia mirror page URL is valid");
    let mut candidates: Vec<OfficialPathDownloadCandidate> = Vec::new();
    let mut cursor = 0;
    while cursor < html.len() {
        let Some(relative_index) = html[cursor..].to_ascii_lowercase().find("href") else { break; };
        let mut position = cursor + relative_index + 4;
        while html.as_bytes().get(position).is_some_and(u8::is_ascii_whitespace) { position += 1; }
        if html.as_bytes().get(position) != Some(&b'=') { cursor = position; continue; }
        position += 1;
        while html.as_bytes().get(position).is_some_and(u8::is_ascii_whitespace) { position += 1; }
        let Some(&quote) = html.as_bytes().get(position) else { break; };
        if quote != b'"' && quote != 39 { cursor = position + 1; continue; }
        let value_start = position + 1;
        let Some(relative_end) = html[value_start..].find(quote as char) else { break; };
        let value_end = value_start + relative_end;
        let raw = html[value_start..value_end].replace("&amp;", "&");
        cursor = value_end + 1;
        let Ok(base) = page.join(&raw) else { continue; };
        let path = base.path().to_ascii_lowercase();
        if base.scheme() != "https"
            || base.host_str().is_none()
            || base.host_str().is_some_and(|host| host == "mirrors.mageia.org" || host.ends_with(".mageia.org"))
            || !base.username().is_empty()
            || base.password().is_some()
            || base.query().is_some()
            || base.fragment().is_some()
            || !(path == "/" || path.ends_with("/mageia/"))
        { continue; }
        let root = if path == "/" {
            match base.join("mageia/") { Ok(url) => url, Err(_) => continue }
        } else { base };
        let Ok(url) = root.join(artifact_path) else { continue; };
        if url.scheme() != "https" || !url.path().ends_with(artifact_path) { continue; }
        let candidate_url = url.to_string();
        if candidates.iter().any(|item| item.url == candidate_url) { continue; }
        candidates.push(OfficialPathDownloadCandidate {
            url: candidate_url,
            provider: "Mageia official mirror database".to_owned(),
            region: url.host_str().unwrap_or("mirror").to_owned(),
        });
        if candidates.len() == 30 { break; }
    }
    if candidates.is_empty() {
        return Err("La base oficial de Mageia no anunció mirrors HTTPS compatibles para esa ISO.".into());
    }
    Ok(candidates)
}

/// Resolve Mageia ISO mirrors from the official mirror database.
pub fn list_mageia_official_download_candidates(
    artifact_path: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    const PAGE_URL: &str = "https://mirrors.mageia.org/";
    let html = fetch_official_mirror_page(
        PAGE_URL,
        "mirrors.mageia.org",
        "VoidBoot-MageiaMirrorResolver/0.1",
    )?;
    parse_mageia_official_mirrors(&html, artifact_path)
}

/// Extract exact KDE neon ISO links from the official live-image page.
pub fn parse_kde_neon_official_download_links(
    html: &str,
    page_url: &str,
    file_name: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    if file_name.is_empty()
        || file_name.len() > 180
        || !file_name.starts_with("neon-")
        || !file_name.to_ascii_lowercase().ends_with(".iso")
        || !file_name.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
    {
        return Err("Indica el nombre exacto de una ISO KDE neon.".into());
    }
    let page = reqwest::Url::parse(page_url)
        .map_err(|error| format!("Página oficial KDE neon inválida: {error}"))?;
    let mut candidates = Vec::new();
    let mut cursor = 0;
    while cursor < html.len() {
        let Some(relative_index) = html[cursor..].to_ascii_lowercase().find("href") else { break; };
        let mut position = cursor + relative_index + 4;
        while html.as_bytes().get(position).is_some_and(u8::is_ascii_whitespace) { position += 1; }
        if html.as_bytes().get(position) != Some(&b'=') { cursor = position; continue; }
        position += 1;
        while html.as_bytes().get(position).is_some_and(u8::is_ascii_whitespace) { position += 1; }
        let Some(&quote) = html.as_bytes().get(position) else { break; };
        if quote != b'"' && quote != 39 { cursor = position + 1; continue; }
        let value_start = position + 1;
        let Some(relative_end) = html[value_start..].find(quote as char) else { break; };
        let value_end = value_start + relative_end;
        let raw = html[value_start..value_end].replace("&amp;", "&");
        cursor = value_end + 1;
        let Ok(url) = page.join(&raw) else { continue; };
        if url.scheme() != "https"
            || url.host_str() != Some("files.kde.org")
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.path().rsplit('/').next() != Some(file_name)
        { continue; }
        let candidate_url = url.to_string();
        if candidates.iter().any(|item: &OfficialPathDownloadCandidate| item.url == candidate_url) { continue; }
        candidates.push(OfficialPathDownloadCandidate {
            url: candidate_url,
            provider: "KDE neon official image host".to_owned(),
            region: "files.kde.org; publisher endpoint may redirect to a mirror".to_owned(),
        });
        if candidates.len() == 10 { break; }
    }
    if candidates.is_empty() {
        return Err("La página oficial KDE neon no publicó un enlace files.kde.org para ese nombre exacto.".into());
    }
    Ok(candidates)
}

/// Resolve KDE neon live images from the publisher's current download page.
pub fn list_kde_neon_official_download_candidates(
    file_name: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    const PAGE_URL: &str = "https://neon.kde.org/download";
    let html = fetch_official_mirror_page(
        PAGE_URL,
        "neon.kde.org",
        "VoidBoot-KDENeonResolver/0.1",
    )?;
    parse_kde_neon_official_download_links(&html, PAGE_URL, file_name)
}

/// Build a Deepin ISO URL on the publisher's official release repository.
pub fn list_deepin_official_download_candidates(
    release: &str,
    architecture: &str,
    file_name: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    let valid_release = !release.is_empty()
        && release.len() <= 20
        && release.bytes().all(|byte| byte.is_ascii_digit() || byte == b'.')
        && !release.starts_with('.')
        && !release.ends_with('.')
        && !release.contains("..");
    if !valid_release {
        return Err("Indica una versión Deepin válida.".into());
    }
    if !["amd64", "arm64", "loong64", "riscv64"].contains(&architecture) {
        return Err("La arquitectura Deepin debe ser amd64, arm64, loong64 o riscv64.".into());
    }
    let expected_name = format!("deepin-desktop-community-{release}-{architecture}.iso");
    if !safe_iso_name(file_name) || file_name != expected_name {
        return Err("El nombre de ISO no coincide con la versión y arquitectura Deepin solicitadas.".into());
    }
    let url = format!("https://cdimage.deepin.com/releases/{release}/{architecture}/{file_name}");
    let parsed = reqwest::Url::parse(&url)
        .map_err(|error| format!("La URL oficial de Deepin es inválida: {error}"))?;
    if parsed.scheme() != "https"
        || parsed.host_str() != Some("cdimage.deepin.com")
        || !parsed.path().ends_with(&format!("/{file_name}"))
    {
        return Err("La URL no pertenece al repositorio HTTPS oficial de Deepin.".into());
    }
    Ok(vec![OfficialPathDownloadCandidate {
        url,
        provider: "Deepin official ISO repository".to_owned(),
        region: "Repositorio oficial de imágenes".to_owned(),
    }])
}

/// Parse Gentoo's signed latest-build pointer for the amd64 minimal installer.
fn parse_gentoo_latest_iso_pointer(text: &str) -> Result<String, String> {
    if !text.contains("-----BEGIN PGP SIGNED MESSAGE-----")
        || !text.contains("-----BEGIN PGP SIGNATURE-----")
        || !text.contains("-----END PGP SIGNATURE-----")
    {
        return Err("El índice latest de Gentoo no tiene el formato de manifiesto firmado esperado.".into());
    }
    let mut matches = text.lines().filter_map(|line| {
        let mut fields = line.split_whitespace();
        let file_name = fields.next()?;
        let size = fields.next()?;
        if fields.next().is_some()
            || !file_name.starts_with("install-amd64-minimal-")
            || !file_name.ends_with(".iso")
            || !safe_iso_name(file_name)
            || size.parse::<u64>().is_err()
        {
            return None;
        }
        Some(file_name.to_owned())
    });
    let file_name = matches.next()
        .ok_or_else(|| "El índice de Gentoo no anunció una ISO amd64 minimal válida.".to_string())?;
    if matches.next().is_some() {
        return Err("El índice de Gentoo anunció más de una ISO amd64 minimal.".into());
    }
    Ok(file_name)
}

/// Resolve the current Gentoo amd64 minimal installer through its official CDN index.
pub fn list_gentoo_official_download_candidates() -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    const INDEX_URL: &str = "https://distfiles-cdn.gentoo.org/releases/amd64/autobuilds/current-install-amd64-minimal/latest-install-amd64-minimal.txt";
    let text = fetch_official_mirror_page(
        INDEX_URL,
        "distfiles-cdn.gentoo.org",
        "VoidBoot-GentooResolver/0.1",
    )?;
    let file_name = parse_gentoo_latest_iso_pointer(&text)?;
    let url = format!(
        "https://distfiles-cdn.gentoo.org/releases/amd64/autobuilds/current-install-amd64-minimal/{file_name}"
    );
    let parsed = reqwest::Url::parse(&url)
        .map_err(|error| format!("La URL oficial de Gentoo es inválida: {error}"))?;
    if parsed.scheme() != "https"
        || parsed.host_str() != Some("distfiles-cdn.gentoo.org")
        || parsed.path().rsplit('/').next() != Some(file_name.as_str())
    {
        return Err("La URL no pertenece al CDN HTTPS oficial de Gentoo.".into());
    }
    Ok(vec![OfficialPathDownloadCandidate {
        url,
        provider: "Gentoo official CDN".to_owned(),
        region: "ISO amd64 minimal más reciente del índice oficial".to_owned(),
    }])
}

/// Resolve elementary OS ISO links from the publisher's live download page.
pub fn list_elementary_official_download_candidates(file_name: &str)
    -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    if !safe_iso_name(file_name) || !file_name.starts_with("elementaryos-")
        || !(file_name.contains("-amd64.") || file_name.contains("-arm64.")) {
        return Err("Indica únicamente el nombre válido de una ISO elementary OS (amd64 o arm64).".into());
    }
    const PAGE_URL: &str = "https://elementary.io/en/";
    let html = fetch_official_mirror_page(PAGE_URL, "elementary.io", "VoidBoot-elementaryOSResolver/0.1")?;
    parse_elementary_official_download_links(&html, PAGE_URL, file_name)
}

/// Parse the exact ISO filename from the mirror links published by EndeavourOS.
fn parse_endeavouros_mirror_links(
    html: &str,
    page_url: &str,
    file_name: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    let page = reqwest::Url::parse(page_url)
        .map_err(|error| format!("Página oficial de EndeavourOS inválida: {error}"))?;
    let mut candidates = Vec::new();
    let mut cursor = 0;

    while cursor < html.len() {
        let Some(relative_index) = html[cursor..].to_ascii_lowercase().find("href") else {
            break;
        };
        let mut position = cursor + relative_index + 4;
        while html
            .as_bytes()
            .get(position)
            .is_some_and(u8::is_ascii_whitespace)
        {
            position += 1;
        }
        if html.as_bytes().get(position) != Some(&b'=') {
            cursor = position;
            continue;
        }
        position += 1;
        while html
            .as_bytes()
            .get(position)
            .is_some_and(u8::is_ascii_whitespace)
        {
            position += 1;
        }
        let Some(&quote) = html.as_bytes().get(position) else {
            break;
        };
        if quote != b'"' && quote != b'\'' {
            cursor = position + 1;
            continue;
        }
        let value_start = position + 1;
        let Some(relative_end) = html[value_start..].find(quote as char) else {
            break;
        };
        let value_end = value_start + relative_end;
        let raw = html[value_start..value_end].replace("&amp;", "&");
        cursor = value_end + 1;

        let Ok(url) = page.join(&raw) else {
            continue;
        };
        if url.scheme() != "https"
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.path().rsplit('/').next() != Some(file_name)
        {
            continue;
        }
        let candidate_url = url.to_string();
        if candidates.iter().any(|item: &OfficialPathDownloadCandidate| item.url == candidate_url) {
            continue;
        }
        candidates.push(OfficialPathDownloadCandidate {
            url: candidate_url,
            provider: "EndeavourOS official download mirror".to_owned(),
            region: url.host_str().unwrap_or("mirror").to_owned(),
        });
        if candidates.len() == 30 {
            break;
        }
    }

    if candidates.is_empty() {
        return Err("La página oficial de EndeavourOS no anunció mirrors HTTPS para ese nombre de ISO.".into());
    }
    Ok(candidates)
}

/// Resolve EndeavourOS ISO mirrors from the publisher's current download page.
pub fn list_endeavouros_official_download_candidates(
    file_name: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    if file_name.is_empty()
        || file_name.len() > 160
        || !file_name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
        || file_name.contains("..")
        || !file_name.to_ascii_lowercase().ends_with(".iso")
    {
        return Err("Indica únicamente el nombre válido de una ISO EndeavourOS.".into());
    }
    const PAGE_URL: &str = "https://endeavouros.com/download/";
    let html = fetch_official_mirror_page(
        PAGE_URL,
        "endeavouros.com",
        "VoidBoot-EndeavourOSMirrorResolver/0.1",
    )?;
    parse_endeavouros_mirror_links(&html, PAGE_URL, file_name)
}

/// Validate a safe relative path ending in an ISO filename.
fn safe_relative_iso_path(artifact_path: &str) -> bool {
    !artifact_path.is_empty()
        && !artifact_path.starts_with('/')
        && !artifact_path.contains("..")
        && !artifact_path.contains('\\')
        && artifact_path
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"/._-".contains(&byte))
        && artifact_path.to_ascii_lowercase().ends_with(".iso")
}

/// Extract only HTTPS mirror links that serve the exact requested openSUSE artifact.
fn parse_opensuse_mirrorlist(
    html: &str,
    artifact_path: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    let base = reqwest::Url::parse("https://download.opensuse.org/")
        .expect("static openSUSE base URL is valid");
    let mut candidates: Vec<OfficialPathDownloadCandidate> = Vec::new();
    let mut cursor = 0;

    while cursor < html.len() {
        let Some(relative_index) = html[cursor..].to_ascii_lowercase().find("href") else {
            break;
        };
        let mut position = cursor + relative_index + 4;
        while html
            .as_bytes()
            .get(position)
            .is_some_and(u8::is_ascii_whitespace)
        {
            position += 1;
        }
        if html.as_bytes().get(position) != Some(&b'=') {
            cursor = position;
            continue;
        }
        position += 1;
        while html
            .as_bytes()
            .get(position)
            .is_some_and(u8::is_ascii_whitespace)
        {
            position += 1;
        }
        let Some(&quote) = html.as_bytes().get(position) else {
            break;
        };
        if quote != b'"' && quote != b'\'' {
            cursor = position + 1;
            continue;
        }
        let value_start = position + 1;
        let Some(relative_end) = html[value_start..].find(quote as char) else {
            break;
        };
        let value_end = value_start + relative_end;
        let raw = html[value_start..value_end].replace("&amp;", "&");
        cursor = value_end + 1;

        let Ok(url) = base.join(&raw) else {
            continue;
        };
        if url.scheme() != "https"
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || !url.path().ends_with(artifact_path)
        {
            continue;
        }
        let candidate_url = url.to_string();
        if candidates.iter().any(|item| item.url == candidate_url) {
            continue;
        }
        candidates.push(OfficialPathDownloadCandidate {
            url: candidate_url,
            provider: "openSUSE MirrorCache".to_owned(),
            region: "Mirror HTTPS oficial".to_owned(),
        });
        if candidates.len() == 20 {
            break;
        }
    }

    if candidates.is_empty() {
        return Err("MirrorCache no anunció mirrors HTTPS válidos para esa ISO exacta.".into());
    }
    Ok(candidates)
}

/// Query openSUSE's official mirrorlist page for the exact ISO artifact.
pub fn list_opensuse_official_download_candidates(
    artifact_path: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    if artifact_path.is_empty()
        || artifact_path.starts_with('/')
        || artifact_path.contains("..")
        || artifact_path.contains('\\')
        || !artifact_path
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"/._-".contains(&byte))
        || !artifact_path.to_ascii_lowercase().ends_with(".iso")
    {
        return Err("Indica una ruta relativa válida de ISO openSUSE, sin rutas absolutas ni segmentos '..'.".into());
    }

    let host = "download.opensuse.org";
    let url = format!("https://{host}/{artifact_path}.mirrorlist");
    let addresses = (host, 443)
        .to_socket_addrs()
        .map_err(|error| format!("No se pudo resolver MirrorCache de openSUSE: {error}"))?
        .collect::<Vec<_>>();
    if addresses.is_empty()
        || addresses
            .iter()
            .any(|address| !crate::download::is_safe_ip(address.ip()))
    {
        return Err("MirrorCache de openSUSE resolvió a una dirección no pública.".into());
    }

    let client = reqwest::blocking::Client::builder()
        .resolve_to_addrs(host, &addresses)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(15))
        .user_agent("VoidBoot-openSUSEMirrorResolver/0.1")
        .build()
        .map_err(|error| format!("No se pudo preparar la consulta a openSUSE: {error}"))?;
    let response = client
        .get(&url)
        .send()
        .map_err(|error| format!("No se pudo consultar MirrorCache de openSUSE: {error}"))?;
    if !response.status().is_success() || response.url().as_str() != url {
        return Err(format!(
            "MirrorCache de openSUSE respondió con {}; no se siguieron redirecciones.",
            response.status()
        ));
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_OPENSUSE_MIRRORLIST_BYTES)
    {
        return Err("La lista de mirrors openSUSE supera el límite de tamaño permitido.".into());
    }
    let mut html = String::new();
    response
        .take(MAX_OPENSUSE_MIRRORLIST_BYTES + 1)
        .read_to_string(&mut html)
        .map_err(|error| format!("No se pudo leer MirrorCache de openSUSE: {error}"))?;
    if html.len() as u64 > MAX_OPENSUSE_MIRRORLIST_BYTES {
        return Err("La lista de mirrors openSUSE supera el límite de tamaño permitido.".into());
    }

    parse_opensuse_mirrorlist(&html, artifact_path)
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DebianDownloadCandidate {
    pub url: String,
    pub provider: String,
    pub region: String,
}

/// Build a Debian ISO URL on Debian's documented CloudFront-backed image mirror.
/// Only relative ISO paths under /debian-cd/ are accepted.
pub fn list_debian_official_download_candidates(
    artifact_path: &str,
) -> Result<Vec<DebianDownloadCandidate>, String> {
    const HOST: &str = "cloudfront.debian.net";

    if artifact_path.is_empty()
        || artifact_path.starts_with('/')
        || artifact_path.contains("..")
        || artifact_path.contains('\\')
        || !artifact_path
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"/._-".contains(&byte))
        || !artifact_path.to_ascii_lowercase().ends_with(".iso")
    {
        return Err(
            "Indica una ruta relativa válida de ISO Debian, sin rutas absolutas ni segmentos '..'."
                .into(),
        );
    }

    let url = format!("https://{HOST}/debian-cd/{artifact_path}");
    let parsed = reqwest::Url::parse(&url)
        .map_err(|error| format!("La ruta de ISO Debian generó una URL inválida: {error}"))?;
    if parsed.scheme() != "https" || parsed.host_str() != Some(HOST) {
        return Err("La URL generada no pertenece al CDN HTTPS oficial de Debian.".into());
    }

    Ok(vec![DebianDownloadCandidate {
        url,
        provider: "Debian CloudFront".to_owned(),
        region: "CDN global".to_owned(),
    }])
}

/// How a distribution obtains ISO download mirrors or an official download source.
/// "directory" means the publisher documents mirrors, but Void Boot does not yet
/// resolve a release-specific ISO URL automatically.
/// Global release gate for Distro Center downloads.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DistroCenterReleaseGate {
    pub downloads_enabled: bool,
    pub download_enabled_systems: usize,
    pub required_systems: usize,
    pub enabled_distribution_ids: Vec<String>,
    pub message: String,
}

// Only distributions with a built-in publisher-hosted SHA-256 manifest adapter are
// eligible for automated downloads. Boot-profile validation is a separate release gate.
const DOWNLOAD_ENABLED_DISTRIBUTIONS: &[&str] = &[
    "debian", "arch", "ubuntu", "lubuntu", "kubuntu", "xubuntu", "fedora", "opensuse",
    "antix", "rocky-linux", "almalinux", "centos-stream",
];

pub fn distro_center_release_gate() -> DistroCenterReleaseGate {
    let enabled_distribution_ids = DOWNLOAD_ENABLED_DISTRIBUTIONS
        .iter()
        .map(|id| (*id).to_owned())
        .collect::<Vec<_>>();
    DistroCenterReleaseGate {
        downloads_enabled: !enabled_distribution_ids.is_empty(),
        download_enabled_systems: enabled_distribution_ids.len(),
        required_systems: 40,
        enabled_distribution_ids,
        message: "Descargas automatizadas habilitadas para 12 distribuciones con adaptadores SHA-256 oficiales: Debian, Arch, Ubuntu y sabores, Fedora, openSUSE, antiX, Rocky Linux, AlmaLinux y CentOS Stream. Oracle Linux permanece bloqueado hasta verificar criptográficamente su firma OpenPGP. Esto no certifica el arranque multiboot; BIOS/UEFI se valida por separado.".to_owned(),
    }
}

/// Require an official, live publisher checksum for the exact ISO before any bytes are downloaded.
pub fn ensure_distro_center_downloads_enabled(
    distribution_id: &str,
    file_name: &str,
    expected_sha256: Option<&str>,
) -> Result<(), String> {
    let gate = distro_center_release_gate();
    if !gate
        .enabled_distribution_ids
        .iter()
        .any(|enabled| enabled == distribution_id)
    {
        return Err(format!(
            "Las descargas automatizadas aún no están habilitadas para {distribution_id}. Solo se permiten sistemas incluidos en la lista de adaptadores SHA-256 oficiales revisados."
        ));
    }

    let expected = expected_sha256
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            "Consulta el SHA-256 oficial en línea antes de descargar esta ISO.".to_string()
        })?;
    if expected.len() != 64 || !expected.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(
            "El SHA-256 oficial esperado debe contener 64 caracteres hexadecimales.".into(),
        );
    }

    let official = lookup_official_release_checksum(distribution_id, file_name)?;
    if !expected.eq_ignore_ascii_case(&official.sha256) {
        return Err(
            "El SHA-256 indicado no coincide con el manifiesto oficial actual. Se bloqueó la descarga."
                .into(),
        );
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MirrorSupport {
    pub distribution_id: String,
    pub mode: String,
    pub label: String,
    pub description: String,
    pub source_url: String,
    pub selectable: bool,
}

pub fn mirror_support_for_distribution(distribution_id: &str) -> Result<MirrorSupport, String> {
    let (mode, label, description, source_url, selectable) = match distribution_id {
        "arch" => (
            "live_status_api",
            "Selector oficial en vivo",
            "La API de estado de Arch entrega mirrors HTTPS activos y sincronizados; Void Boot puede filtrarlos y seleccionarlos.",
            "https://archlinux.org/mirrors/status/",
            true,
        ),
        "debian" => (
            "official_cdn",
            "CDN oficial de imágenes Debian",
            "Void Boot genera enlaces HTTPS mediante cloudfront.debian.net, CDN documentado por Debian para distribuir /debian-cd/. La ruta de la ISO se especifica desde el índice oficial.",
            "https://cloudfront.debian.net/debian-cd/",
            true,
        ),
        "ubuntu" | "lubuntu" | "kubuntu" | "xubuntu" => (
            "official_release_directory",
            "Descargas oficiales de Ubuntu",
            "Void Boot construye destinos HTTPS oficiales a partir de la versión y el nombre exacto: releases.ubuntu.com para Ubuntu y cdimage.ubuntu.com para Lubuntu, Kubuntu y Xubuntu. No se presenta como selector de mirrors.",
            "https://releases.ubuntu.com/",
            false,
        ),
        "kali" => (
            "official_redirector",
            "Redirector oficial de imágenes de Kali",
            "cdimage.kali.org selecciona un mirror cercano y anuncia alternativas HTTPS oficiales para la ISO exacta; Void Boot puede consultar esos candidatos.",
            "https://cdimage.kali.org/",
            true,
        ),
        "void-linux" => (
            "official_current_image_tree",
            "Repositorio oficial de imágenes Void Linux",
            "Void Boot construye rutas exactas únicamente dentro de live/current/ en repo-default.voidlinux.org. El proyecto publica mirrors adicionales, pero este resolver usa el endpoint oficial predeterminado y no los presenta como selector.",
            "https://voidlinux.org/download/",
            false,
        ),
        "mx-linux" => (
            "official_download_page",
            "Descarga oficial de MX Linux",
            "MX Linux publica sus propios enlaces de descarga. No se asume que los mirrors de paquetes Debian contengan sus ISOs.",
            "https://mxlinux.org/download-links/",
            false,
        ),
        "mx-linux" => (
            "publisher_live_iso_mirror_status",
            "Mirrors oficiales de ISO MX Linux",
            "Void Boot consulta el índice mirmon publicado por MX Linux y acepta únicamente raíces HTTPS anunciadas por el proyecto que terminan en /iso/; exige la ruta exacta dentro de MX/Final/ o MX/Snapshots/.",
            "https://rsync-mxlinux.org/mirmon/index.html",
            true,
        ),
        "mageia" => (
            "official_mirror_database",
            "Base oficial de mirrors Mageia",
            "Void Boot consulta mirrors.mageia.org y acepta solo destinos HTTPS anunciados por Mageia cuya raíz es / o termina en /mageia/; construye la ruta exacta dentro de iso/.",
            "https://mirrors.mageia.org/",
            true,
        ),
        "deepin" => (
            "official_download_page",
            "Descarga oficial de deepin",
            "Se utiliza el canal de descarga del proyecto; no se reutilizan mirrors Debian para imágenes ISO.",
            "https://www.deepin.org/en/download/",
            false,
        ),
        "antix" => (
            "publisher_live_iso_links",
            "Mirrors ISO publicados por antiX",
            "Void Boot consulta la página oficial y conserva únicamente enlaces HTTPS anunciados para el nombre exacto de la ISO; no reutiliza mirrors de repositorios de paquetes.",
            "https://antixlinux.com/download/",
            false,
        ),
        "linux-mint" => (
            "official_mirror_directory",
            "Mirrors oficiales de ISO Linux Mint",
            "Void Boot lee el listado oficial de mirrors de Linux Mint y conserva únicamente enlaces HTTPS cuya base apunta al directorio /iso/, excluyendo mirrors de paquetes.",
            "https://www.linuxmint.com/mirrors.php",
            true,
        ),
        "zorin-os" => (
            "publisher_listed_iso_mirror",
            "Mirror ISO publicado por Zorin OS",
            "Void Boot construye la ruta de ISO en Kernel.org, mirror global enlazado explícitamente por Zorin; el resolver valida la versión y el nombre exacto del archivo.",
            "https://zorin.com/os/mirrors/",
            false,
        ),
        "pop-os" => (
            "official_iso_host",
            "CDN oficial de imágenes Pop!_OS",
            "Void Boot construye la ruta exacta en iso.pop-os.org para builds amd64 genéricos o NVIDIA; la página oficial de System76 publica los checksums y las variantes disponibles.",
            "https://system76.com/download-pop/",
            false,
        ),
        "elementary-os" => (
            "publisher_dynamic_download",
            "Gateway oficial de descargas elementary OS",
            "Void Boot consulta la página oficial en vivo y extrae únicamente el enlace HTTPS de dl.elementaryos.org cuyo nombre coincide exactamente con la ISO solicitada.",
            "https://elementary.io/en/",
            false,
        ),
        "linux-lite" => (
            "publisher_live_iso_links",
            "Descargas ISO oficiales Linux Lite",
            "Void Boot consulta la página oficial y extrae enlaces HTTPS directos cuyo nombre coincide exactamente con la ISO solicitada; no reutiliza mirrors de paquetes Ubuntu.",
            "https://www.linuxliteos.com/download.php",
            false,
        ),
        "kde-neon" => (
            "official_download_page",
            "Descarga oficial de KDE neon",
            "Se conserva el canal oficial de KDE neon; los mirrors de paquetes Ubuntu no se tratan como mirrors de ISO.",
            "https://neon.kde.org/download",
            false,
        ),
        "kde-neon" => (
            "publisher_live_image_links",
            "Imágenes live oficiales KDE neon",
            "Void Boot lee la página oficial y acepta únicamente enlaces HTTPS files.kde.org cuyo nombre coincide exactamente con la ISO solicitada; el endpoint puede redirigir a un mirror externo del proyecto.",
            "https://neon.kde.org/download",
            false,
        ),
        "manjaro" => (
            "official_download_host",
            "Servidor oficial de descargas Manjaro",
            "Void Boot construye rutas de ISO para las ediciones oficiales GNOME, KDE y Xfce en download.manjaro.org, sin utilizar mirrors de repositorios como si fueran mirrors de instalación.",
            "https://download.manjaro.org/",
            false,
        ),
        "endeavouros" => (
            "publisher_mirror_directory",
            "Mirrors publicados por EndeavourOS",
            "Void Boot consulta la página oficial de descargas y extrae únicamente enlaces HTTPS que coinciden exactamente con el nombre de ISO solicitado.",
            "https://endeavouros.com/download/",
            false,
        ),
        "centos-stream" => (
            "official_mirror_host",
            "Mirror oficial CentOS Stream",
            "Void Boot construye la ruta ISO exacta en mirror.stream.centos.org, host enlazado por el sitio oficial del proyecto; admite los árboles BaseOS x86_64 y aarch64 de Stream 9 y 10.",
            "https://mirror.stream.centos.org/",
            false,
        ),
        "oracle-linux" => (
            "official_iso_host",
            "Servidor oficial Oracle Linux",
            "Void Boot construye rutas ISO exactas en yum.oracle.com, host oficial de imágenes de Oracle Linux, y restringe las rutas a OL9/OL10 y arquitecturas x86_64/aarch64.",
            "https://yum.oracle.com/ISOS/OracleLinux/",
            false,
        ),
        "rocky-linux" => (
            "official_download_host",
            "Servidor oficial de Rocky Linux",
            "Void Boot construye rutas ISO relativas en download.rockylinux.org y restringe las rutas admitidas a los árboles de versiones 9 y 10.",
            "https://download.rockylinux.org/pub/rocky/",
            false,
        ),
        "almalinux" => (
            "official_cdn",
            "CDN oficial de AlmaLinux",
            "Void Boot construye la ruta exacta en repo.almalinux.org, servidor maestro oficial de AlmaLinux respaldado por CDN; no mezcla mirrors comunitarios ni estados caducados.",
            "https://repo.almalinux.org/almalinux/",
            false,
        ),
        "fedora" => (
            "official_mirrormanager",
            "Fedora MirrorManager",
            "Void Boot consulta el redirector oficial de Fedora para una ISO exacta de Workstation x86_64 y acepta únicamente un destino HTTPS cuya ruta coincida con el artefacto solicitado.",
            "https://download.fedoraproject.org/",
            true,
        ),
        "opensuse" => (
            "official_metalink",
            "MirrorCache / Metalink oficial",
            "Void Boot consulta la página .mirrorlist oficial para la ruta exacta de la ISO y extrae únicamente destinos HTTPS que conservan el mismo artefacto.",
            "https://download.opensuse.org/",
            true,
        ),
        "freebsd" => (
            "official_geodns",
            "GeoDNS / CDN oficial",
            "Void Boot construye la ruta HTTPS del artefacto en download.freebsd.org; la infraestructura oficial GeoDNS/CDN dirige la descarga sin simular un selector manual de mirrors.",
            "https://download.freebsd.org/",
            true,
        ),
        "openbsd" => (
            "official_mirror_directory",
            "Directorio oficial de mirrors",
            "Void Boot lee la lista publicada por OpenBSD y combina las bases HTTPS oficiales con la ruta relativa exacta de la ISO solicitada.",
            "https://www.openbsd.org/ftp.html",
            true,
        ),
        "netbsd" => (
            "official_mirror_directory",
            "Directorio oficial de mirrors",
            "Void Boot lee la lista publicada por NetBSD y combina las bases HTTPS oficiales con la ruta relativa exacta de la ISO solicitada.",
            "https://www.netbsd.org/mirrors/",
            true,
        ),
        "slackware" => (
            "official_iso_index",
            "Índice oficial de ISOs Slackware",
            "Void Boot construye destinos HTTPS únicamente dentro del índice ISO publicado por mirrors.slackware.com para Slackware 15.0, separando las imágenes de 32 y 64 bits.",
            "https://mirrors.slackware.com/slackware/slackware-iso/",
            false,
        ),
        "solus" => (
            "publisher_live_iso_links",
            "Imágenes oficiales Solus",
            "Void Boot consulta la página oficial y acepta únicamente enlaces HTTPS de downloads.getsol.us cuyo nombre coincide exactamente con la ISO de la edición solicitada.",
            "https://getsol.us/download/",
            false,
        ),
        "windows-11-x64" => (
            "official_single_source",
            "Windows 11 x64 · ISO multi-edición oficial",
            "Acceso directo a la página oficial de Microsoft para Windows 11 x64. Microsoft genera enlaces de ISO firmados y temporales tras seleccionar edición e idioma; Void Boot no los almacena como destinos permanentes.",
            "https://www.microsoft.com/software-download/windows11",
            false,
        ),
        "windows-11-arm64" => (
            "official_single_source",
            "Windows 11 ARM64 · ISO oficial",
            "Acceso directo a la página oficial específica de Windows 11 ARM64. No se reutiliza la ISO x64 ni se ofrecen mirrors de terceros; los enlaces de descarga son temporales.",
            "https://www.microsoft.com/software-download/windows11arm64",
            false,
        ),
        "windows-10-home" | "windows-10-pro" => (
            "official_single_source",
            "Windows 10 Home/Pro · ISO multi-edición",
            "Home y Pro comparten el medio multi-edición oficial de Windows 10. La edición instalada depende de la licencia; Microsoft genera enlaces temporales después de seleccionar idioma.",
            "https://www.microsoft.com/en-us/software-download/windows10ISO",
            false,
        ),
        "windows-10-ltsc" => (
            "official_single_source",
            "Windows 10 Enterprise LTSC · Evaluación oficial",
            "Acceso al Centro de evaluación oficial de Microsoft para ISO Enterprise LTSC. La evaluación tiene condiciones y duración propias; el uso permanente requiere licencia válida.",
            "https://www.microsoft.com/en-us/evalcenter/download-windows-10-enterprise",
            false,
        ),
        "reactos" => (
            "official_nightly_iso_archive",
            "ReactOS · Archivo oficial BuildBot",
            "Void Boot puede resolver una ruta ISO exacta del archivo oficial iso.reactos.org/bootcd/ o /livecd/. Son builds nocturnos de prueba, no el lanzamiento estable; pueden contener regresiones.",
            "https://reactos.org/getbuilds/index.php?lang=en",
            false,
        ),
        "alpine" => (
            "official_iso_cdn",
            "CDN oficial de Alpine Linux",
            "Void Boot construye rutas ISO dentro del árbol latest-stable/releases del CDN oficial y valida arquitectura y nombre de archivo.",
            "https://www.alpinelinux.org/downloads/",
            false,
        ),
        "nixos" => (
            "official_channel_alias",
            "Canales oficiales de NixOS",
            "Void Boot construye alias oficiales de channels.nixos.org para ISOs graphical/minimal de canales estables o unstable en x86_64 y aarch64.",
            "https://nixos.org/download/",
            false,
        ),
        "deepin" => (
            "official_release_iso_repository",
            "Repositorio oficial de ISOs Deepin",
            "Void Boot construye la ruta exacta en cdimage.deepin.com/releases y valida versión, arquitectura y nombre de archivo; admite amd64, arm64, loong64 y riscv64.",
            "https://www.deepin.org/en/mirrors/",
            false,
        ),
        "gentoo" => (
            "official_live_build_index",
            "Índice oficial de ISOs Gentoo",
            "Void Boot consulta el índice latest firmado por Gentoo y obtiene el nombre exacto de la ISO amd64 minimal actual desde su CDN oficial.",
            "https://distfiles-cdn.gentoo.org/releases/amd64/autobuilds/current-install-amd64-minimal/",
            false,
        ),
        "endeavouros" | "manjaro" | "void-linux" | "rocky-linux" | "almalinux" | "mageia" | "centos-stream" | "slackware" | "solus" | "oracle-linux" => (
            "official_download_page",
            "Descarga oficial del proyecto",
            "Este proyecto mantiene su propia infraestructura de distribución. Void Boot todavía no dispone de un índice de mirrors ISO verificable para esta distribución; usa su página oficial.",
            match distribution_id {
                "endeavouros" => "https://endeavouros.com/download/",
                "manjaro" => "https://manjaro.org/products/",
                "nixos" => "https://nixos.org/download/",
                "void-linux" => "https://voidlinux.org/download/",
                "rocky-linux" => "https://rockylinux.org/download",
                "almalinux" => "https://almalinux.org/get-almalinux/",
                "alpine" => "https://alpinelinux.org/downloads/",
                "mageia" => "https://www.mageia.org/downloads/",
                "centos-stream" => "https://www.centos.org/download/",
                "gentoo" => "https://www.gentoo.org/downloads/",
                "slackware" => "https://www.slackware.com/getslack/",
                "solus" => "https://getsol.us/download/",
                _ => "https://www.oracle.com/linux/",
            },
            false,
        ),
        _ => return Err("La distribución no tiene una política de mirrors registrada.".into()),
    };

    Ok(MirrorSupport {
        distribution_id: distribution_id.to_owned(),
        mode: mode.to_owned(),
        label: label.to_owned(),
        description: description.to_owned(),
        source_url: source_url.to_owned(),
        selectable,
    })
}



/// Parse exact ISO links advertised by a publisher's official download page.
/// Only HTTPS links whose final path component is the requested filename survive.
fn parse_publisher_iso_links(
    html: &str,
    page_url: &str,
    file_name: &str,
    allowed_domains: &[&str],
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    if !safe_iso_name(file_name) || file_name.len() > 180 {
        return Err("Indica el nombre exacto de una ISO válida.".into());
    }
    let page = reqwest::Url::parse(page_url)
        .map_err(|error| format!("Página oficial inválida: {error}"))?;
    let mut candidates = Vec::new();
    let mut cursor = 0;
    while cursor < html.len() {
        let Some(relative_index) = html[cursor..].to_ascii_lowercase().find("href") else {
            break;
        };
        let mut position = cursor + relative_index + 4;
        while html.as_bytes().get(position).is_some_and(u8::is_ascii_whitespace) {
            position += 1;
        }
        if html.as_bytes().get(position) != Some(&b'=') {
            cursor = position;
            continue;
        }
        position += 1;
        while html.as_bytes().get(position).is_some_and(u8::is_ascii_whitespace) {
            position += 1;
        }
        let Some(&quote) = html.as_bytes().get(position) else {
            break;
        };
        if quote != b'"' && quote != 39 {
            cursor = position + 1;
            continue;
        }
        let value_start = position + 1;
        let Some(relative_end) = html[value_start..].find(quote as char) else {
            break;
        };
        let value_end = value_start + relative_end;
        let raw = html[value_start..value_end].replace("&amp;", "&");
        cursor = value_end + 1;

        let Ok(url) = page.join(&raw) else {
            continue;
        };
        let Some(host) = url.host_str() else {
            continue;
        };
        let domain_allowed = allowed_domains.is_empty()
            || allowed_domains.iter().any(|domain| {
                host.eq_ignore_ascii_case(domain)
                    || host.to_ascii_lowercase().ends_with(&format!(".{domain}"))
            });
        if url.scheme() != "https"
            || !domain_allowed
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.path().rsplit('/').next() != Some(file_name)
        {
            continue;
        }
        let candidate = OfficialPathDownloadCandidate {
            url: url.to_string(),
            provider: host.to_owned(),
            region: "Enlace HTTPS anunciado por el editor".to_owned(),
        };
        if !candidates.iter().any(|existing: &OfficialPathDownloadCandidate| {
            existing.url == candidate.url
        }) {
            candidates.push(candidate);
        }
    }
    if candidates.is_empty() {
        return Err(format!(
            "La página oficial no anunció un enlace HTTPS directo para {file_name}."
        ));
    }
    candidates.truncate(20);
    Ok(candidates)
}


/// Resolve an exact ReactOS nightly BootCD or LiveCD ISO from the official BuildBot archive.
/// This is explicitly a nightly channel, not a stable-release ISO.
pub fn list_reactos_official_download_candidates(
    artifact_path: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    if !safe_relative_iso_path(artifact_path)
        || artifact_path.matches('/').count() != 1
        || !(artifact_path.starts_with("bootcd/") || artifact_path.starts_with("livecd/"))
    {
        return Err(
            "ReactOS acepta únicamente una ruta ISO exacta dentro de bootcd/ o livecd/ del archivo oficial."
                .into(),
        );
    }
    let file_name = artifact_path.rsplit('/').next().unwrap_or_default();
    if !safe_iso_name(file_name) {
        return Err("El archivo ReactOS debe ser una ISO con nombre válido.".into());
    }
    let url = format!("https://iso.reactos.org/{artifact_path}");
    let parsed = reqwest::Url::parse(&url)
        .map_err(|error| format!("La URL oficial de ReactOS es inválida: {error}"))?;
    if parsed.scheme() != "https"
        || parsed.host_str() != Some("iso.reactos.org")
        || parsed.path() != format!("/{artifact_path}")
    {
        return Err("La ruta no pertenece al archivo HTTPS oficial de ReactOS.".into());
    }
    Ok(vec![OfficialPathDownloadCandidate {
        url,
        provider: "ReactOS official BuildBot ISO archive".to_owned(),
        region: if artifact_path.starts_with("bootcd/") {
            "Build nocturno BootCD; puede contener regresiones".to_owned()
        } else {
            "Build nocturno LiveCD; puede contener regresiones".to_owned()
        },
    }])
}

/// Resolve Slackware 15.0 installer DVDs from the project's official ISO index.
pub fn list_slackware_official_download_candidates(
    artifact_path: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    if !safe_relative_iso_path(artifact_path)
        || !(artifact_path.starts_with("slackware-iso/slackware64-15.0-iso/")
            || artifact_path.starts_with("slackware-iso/slackware-15.0-iso/"))
    {
        return Err(
            "La ruta Slackware debe apuntar al directorio ISO oficial de Slackware 15.0 (32 o 64 bits)."
                .into(),
        );
    }
    let url = format!("https://mirrors.slackware.com/slackware/{artifact_path}");
    let parsed = reqwest::Url::parse(&url)
        .map_err(|error| format!("La URL oficial de Slackware es inválida: {error}"))?;
    if parsed.scheme() != "https"
        || parsed.host_str() != Some("mirrors.slackware.com")
        || !parsed.path().ends_with(".iso")
    {
        return Err("La URL no pertenece al índice HTTPS oficial de Slackware.".into());
    }
    Ok(vec![OfficialPathDownloadCandidate {
        url,
        provider: "Slackware official ISO mirror".to_owned(),
        region: "Mirror oficial del proyecto".to_owned(),
    }])
}

/// Resolve the exact Linux Lite ISO link advertised by its official download page.
pub fn list_linux_lite_official_download_candidates(
    file_name: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    if !safe_iso_name(file_name) || !file_name.starts_with("linux-lite-") {
        return Err("Indica el nombre exacto de una ISO Linux Lite.".into());
    }
    const PAGE_URL: &str = "https://www.linuxliteos.com/download.php";
    let html = fetch_official_mirror_page(
        PAGE_URL,
        "www.linuxliteos.com",
        "VoidBoot-LinuxLiteResolver/0.1",
    )?;
    parse_publisher_iso_links(
        &html,
        PAGE_URL,
        file_name,
        &["linuxliteos.com", "sourceforge.net"],
    )
}

/// Resolve one exact Solus edition ISO from the publisher's live download page.
pub fn list_solus_official_download_candidates(
    file_name: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    if !safe_iso_name(file_name) || !file_name.starts_with("Solus-") {
        return Err("Indica el nombre exacto de una ISO Solus.".into());
    }
    const PAGE_URL: &str = "https://getsol.us/download/";
    let html = fetch_official_mirror_page(
        PAGE_URL,
        "getsol.us",
        "VoidBoot-SolusResolver/0.1",
    )?;
    parse_publisher_iso_links(
        &html,
        PAGE_URL,
        file_name,
        &["downloads.getsol.us"],
    )
}

/// Resolve antiX ISO links explicitly advertised on the project's download page.
/// The page itself curates the mirror destinations; only exact HTTPS ISO filenames are accepted.
pub fn list_antix_official_download_candidates(
    file_name: &str,
) -> Result<Vec<OfficialPathDownloadCandidate>, String> {
    if !safe_iso_name(file_name) || !file_name.starts_with("antiX-") {
        return Err("Indica el nombre exacto de una ISO antiX.".into());
    }
    const PAGE_URL: &str = "https://antixlinux.com/download/";
    let html = fetch_official_mirror_page(
        PAGE_URL,
        "antixlinux.com",
        "VoidBoot-antiXResolver/0.1",
    )?;
    parse_publisher_iso_links(&html, PAGE_URL, file_name, &[])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kde_neon_parser_accepts_only_exact_publisher_iso_links() {
        let html = r#"
            <a href="https://files.kde.org/neon/images/desk/user/neon-user-desk.iso">ISO</a>
            <a href="https://files.kde.org/neon/images/desk/user/neon-user-desk.iso.sig">Signature</a>
            <a href="http://files.kde.org/neon/images/desk/user/neon-user-desk.iso">Insecure</a>
            <a href="https://evil.example/neon-user-desk.iso">Other host</a>
        "#;
        let candidates = parse_kde_neon_official_download_links(
            html,
            "https://neon.kde.org/download",
            "neon-user-desk.iso",
        ).unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].url,
            "https://files.kde.org/neon/images/desk/user/neon-user-desk.iso");
        assert!(parse_kde_neon_official_download_links(
            html, "https://neon.kde.org/download", "../neon.iso"
        ).is_err());
    }

    #[test]
    fn mageia_parser_accepts_only_officially_listed_https_mirror_roots() {
        let html = r#"
            <a href="https://mirror.example/">Mirror root</a>
            <a href="https://other.example/mageia/">Mageia root</a>
            <a href="http://insecure.example/">Insecure</a>
            <a href="https://www.mageia.org/">Project website</a>
        "#;
        let candidates = parse_mageia_official_mirrors(
            html,
            "iso/10/Mageia-10-x86_64/Mageia-10-x86_64.iso",
        ).unwrap();
        assert_eq!(candidates.len(), 2);
        assert!(candidates.iter().any(|candidate| candidate.url ==
            "https://mirror.example/mageia/iso/10/Mageia-10-x86_64/Mageia-10-x86_64.iso"));
        assert!(candidates.iter().any(|candidate| candidate.url ==
            "https://other.example/mageia/iso/10/Mageia-10-x86_64/Mageia-10-x86_64.iso"));
        assert!(parse_mageia_official_mirrors(html, "../private.iso").is_err());
    }

    #[test]
    fn mx_linux_parser_uses_only_publisher_listed_https_iso_roots() {
        let html = r#"
            <a href="https://mirror.example/pub/mxlinux/iso/">Mirror A</a>
            <a href="http://insecure.example/mxlinux/iso/">Insecure</a>
            <a href="https://mirror.example/debian/">Debian packages</a>
        "#;
        let candidates = parse_mx_official_iso_mirrors(
            html,
            "MX/Final/MX-25.3_Xfce_x64.iso",
        ).unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].url,
            "https://mirror.example/pub/mxlinux/iso/MX/Final/MX-25.3_Xfce_x64.iso"
        );
        assert!(parse_mx_official_iso_mirrors(
            html,
            "../MX-25.3_Xfce_x64.iso"
        ).is_err());
        assert!(parse_mx_official_iso_mirrors(
            html,
            "MX/Final/Ubuntu.iso"
        ).is_err());
    }

    #[test]
    fn void_linux_resolver_accepts_only_official_current_live_iso_paths() {
        let candidates = list_void_linux_official_download_candidates(
            "live/current/void-live-x86_64-20250202-base.iso",
        ).unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].url,
            "https://repo-default.voidlinux.org/live/current/void-live-x86_64-20250202-base.iso"
        );
        assert!(list_void_linux_official_download_candidates("../private.iso").is_err());
        assert!(list_void_linux_official_download_candidates(
            "live/current/void-live-x86_64-20250202-unofficial.iso"
        ).is_err());
        assert!(list_void_linux_official_download_candidates(
            "live/20250202/void-live-x86_64-20250202-base.iso"
        ).is_err());
    }

    #[test]
    fn linux_mint_mirror_parser_excludes_repository_mirrors() {
        let html = r#"
            <a href="https://mirror.example/linuxmint/iso/">ISO images</a>
            <a href="https://mirror.example/linuxmint/packages/">Repository packages</a>
        "#;
        let candidates = parse_publisher_mirror_directory(
            html,
            "https://www.linuxmint.com/mirrors.php",
            "stable/22.3/linuxmint-22.3-cinnamon-64bit.iso",
            &["/iso/"],
            "Linux Mint",
        )
        .unwrap();
        assert_eq!(candidates.len(), 1);
        assert!(candidates[0]
            .url
            .ends_with("/linuxmint/iso/stable/22.3/linuxmint-22.3-cinnamon-64bit.iso"));
    }

    #[test]
    fn freebsd_resolver_uses_official_geodns_host_and_rejects_unsafe_paths() {
        let candidates = list_freebsd_official_download_candidates(
            "releases/ISO-IMAGES/15.1/FreeBSD-15.1-RELEASE-amd64-disc1.iso",
        )
        .unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].provider, "FreeBSD GeoDNS / CDN");
        assert!(candidates[0]
            .url
            .starts_with("https://download.freebsd.org/releases/"));
        assert!(list_freebsd_official_download_candidates("../private.iso").is_err());
        assert!(list_freebsd_official_download_candidates("/absolute.iso").is_err());
    }

    #[test]
    fn almalinux_resolver_uses_only_official_cdn_and_versioned_iso_paths() {
        let candidates = list_almalinux_official_download_candidates(
            "10/isos/x86_64/AlmaLinux-10-x86_64-dvd.iso",
        )
        .unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].url,
            "https://repo.almalinux.org/almalinux/10/isos/x86_64/AlmaLinux-10-x86_64-dvd.iso"
        );
        assert!(list_almalinux_official_download_candidates("../private.iso").is_err());
        assert!(list_almalinux_official_download_candidates("8/isos/x86_64/old.iso").is_err());
    }

    #[test]
    fn manjaro_resolver_builds_only_official_edition_iso_urls() {
        let candidates = list_manjaro_official_download_candidates(
            "gnome",
            "26.1.3",
            "manjaro-gnome-26.1.3-261028-linux72.iso",
        )
        .unwrap();
        assert_eq!(
            candidates[0].url,
            "https://download.manjaro.org/gnome/26.1.3/manjaro-gnome-26.1.3-261028-linux72.iso"
        );
        assert!(list_manjaro_official_download_candidates(
            "../private", "26.1.3", "installer.iso"
        )
        .is_err());
        assert!(list_manjaro_official_download_candidates(
            "sway", "26.1.3", "installer.iso"
        )
        .is_err());
    }

    #[test]
    fn zorin_resolver_uses_only_the_official_listed_kernel_mirror() {
        let candidates = list_zorin_official_download_candidates(
            "18",
            "Zorin-OS-18.1-Core-64-bit.iso",
        )
        .unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].url,
            "https://mirrors.edge.kernel.org/zorinos-isos/18/Zorin-OS-18.1-Core-64-bit.iso"
        );
        assert!(list_zorin_official_download_candidates(
            "../18",
            "Zorin-OS-18-Core-64-bit.iso"
        ).is_err());
        assert!(list_zorin_official_download_candidates(
            "18",
            "../private.iso"
        ).is_err());
        assert!(list_zorin_official_download_candidates(
            "18",
            "Ubuntu-18.iso"
        ).is_err());
    }

    #[test]
    fn popos_resolver_builds_only_official_system76_iso_paths() {
        let candidates = list_popos_official_download_candidates(
            "24.04", "amd64", "generic", "22", "pop-os_24.04_amd64_generic_22.iso"
        ).unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].url,
            "https://iso.pop-os.org/24.04/amd64/generic/22/pop-os_24.04_amd64_generic_22.iso");
        assert!(list_popos_official_download_candidates(
            "24.04", "amd64", "unknown", "22", "pop-os_24.04_amd64_unknown_22.iso"
        ).is_err());
        assert!(list_popos_official_download_candidates(
            "../24.04", "amd64", "generic", "22", "pop-os_24.04_amd64_generic_22.iso"
        ).is_err());
        assert!(list_popos_official_download_candidates(
            "24.04", "amd64", "generic", "22", "another.iso"
        ).is_err());
    }

    #[test]
    fn alpine_resolver_uses_only_official_latest_stable_iso_tree() {
        let candidates = list_alpine_official_download_candidates(
            "latest-stable/releases/x86_64/alpine-standard-3.24.4-x86_64.iso",
        ).unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].url,
            "https://dl-cdn.alpinelinux.org/alpine/latest-stable/releases/x86_64/alpine-standard-3.24.4-x86_64.iso");
        assert!(list_alpine_official_download_candidates("../private.iso").is_err());
        assert!(list_alpine_official_download_candidates(
            "latest-stable/releases/x86_64/alpine-standard-3.24.4-aarch64.iso"
        ).is_err());
    }

    #[test]
    fn nixos_resolver_uses_official_channel_aliases_and_validates_selection() {
        let candidates = list_nixos_official_download_candidates(
            "nixos-26.05", "graphical", "x86_64",
        ).unwrap();
        assert_eq!(candidates[0].url,
            "https://channels.nixos.org/nixos-26.05/latest-nixos-graphical-x86_64-linux.iso");
        assert!(list_nixos_official_download_candidates(
            "../evil", "graphical", "x86_64"
        ).is_err());
        assert!(list_nixos_official_download_candidates(
            "nixos-26.05", "desktop", "x86_64"
        ).is_err());
        assert!(list_nixos_official_download_candidates(
            "nixos-26.05", "minimal", "i686"
        ).is_err());
    }

    #[test]
    fn deepin_resolver_builds_only_exact_official_release_iso_paths() {
        let candidates = list_deepin_official_download_candidates(
            "25.3.0", "amd64", "deepin-desktop-community-25.3.0-amd64.iso",
        ).unwrap();
        assert_eq!(candidates[0].url,
            "https://cdimage.deepin.com/releases/25.3.0/amd64/deepin-desktop-community-25.3.0-amd64.iso");
        assert!(list_deepin_official_download_candidates(
            "../25.3", "amd64", "deepin-desktop-community-25.3-amd64.iso"
        ).is_err());
        assert!(list_deepin_official_download_candidates(
            "25.3.0", "amd64", "other.iso"
        ).is_err());
        assert!(list_deepin_official_download_candidates(
            "25.3.0", "i686", "deepin-desktop-community-25.3.0-i686.iso"
        ).is_err());
    }

    #[test]
    fn gentoo_latest_pointer_accepts_only_one_signed_amd64_minimal_iso_entry() {
        let pointer = "-----BEGIN PGP SIGNED MESSAGE-----\\nHash: SHA256\\n# Latest\\ninstall-amd64-minimal-20261115T170104Z.iso 1051310080\\n-----BEGIN PGP SIGNATURE-----\\nsignature\\n-----END PGP SIGNATURE-----";
        assert_eq!(
            parse_gentoo_latest_iso_pointer(pointer).unwrap(),
            "install-amd64-minimal-20261115T170104Z.iso"
        );
        assert!(parse_gentoo_latest_iso_pointer(
            "install-amd64-minimal-20261115T170104Z.iso 100"
        ).is_err());
        assert!(parse_gentoo_latest_iso_pointer(
            "-----BEGIN PGP SIGNED MESSAGE-----\\ninstall-amd64-minimal-20261115T170104Z.iso 100\\ninstall-amd64-minimal-20261116T170104Z.iso 100\\n-----BEGIN PGP SIGNATURE-----\\nx\\n-----END PGP SIGNATURE-----"
        ).is_err());
        assert!(parse_gentoo_latest_iso_pointer(
            "-----BEGIN PGP SIGNED MESSAGE-----\\n../evil.iso 100\\n-----BEGIN PGP SIGNATURE-----\\nx\\n-----END PGP SIGNATURE-----"
        ).is_err());
    }

    #[test]
    fn elementary_parser_accepts_only_exact_publisher_download_links() {
        let html = r#"
            <a href="https://dl.elementaryos.org/token/elementaryos-8.1-stable-amd64.20260219.iso">x86</a>
            <a href="http://dl.elementaryos.org/token/elementaryos-8.1-stable-amd64.20260219.iso">insecure</a>
            <a href="https://evil.example/elementaryos-8.1-stable-amd64.20260219.iso">foreign</a>
            <a href="https://dl.elementaryos.org/token/other.iso">wrong file</a>
            <a href="https://dl.elementaryos.org/token/elementaryos-8.1-stable-amd64.20260219.iso?download=1">query</a>
        "#;
        let candidates = parse_elementary_official_download_links(
            html, "https://elementary.io/en/", "elementaryos-8.1-stable-amd64.20260219.iso"
        ).unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].url,
            "https://dl.elementaryos.org/token/elementaryos-8.1-stable-amd64.20260219.iso");
        assert!(parse_elementary_official_download_links(
            "<a href='https://evil.example/other.iso'>x</a>",
            "https://elementary.io/en/", "elementaryos-8.1-stable-amd64.20260219.iso"
        ).is_err());
        assert!(list_elementary_official_download_candidates("../private.iso").is_err());
    }

    #[test]
    fn endeavouros_parser_accepts_only_exact_https_iso_links() {
        let html = r#"
            <a href="https://mirror.example/endeavouros/EndeavourOS.iso">ISO</a>
            <a href="https://mirror.example/endeavouros/EndeavourOS.iso.sha512sum">Checksum</a>
            <a href="http://insecure.example/EndeavourOS.iso">Insecure</a>
            <a href="https://other.example/Other.iso">Other image</a>
        "#;
        let candidates = parse_endeavouros_mirror_links(
            html,
            "https://endeavouros.com/download/",
            "EndeavourOS.iso",
        )
        .unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].url,
            "https://mirror.example/endeavouros/EndeavourOS.iso"
        );
    }

    #[test]
    fn centos_stream_resolver_uses_only_official_iso_trees() {
        let candidates = list_centos_stream_official_download_candidates(
            "9-stream/BaseOS/x86_64/iso/CentOS-Stream-9-latest-x86_64-boot.iso",
        )
        .unwrap();
        assert_eq!(
            candidates[0].url,
            "https://mirror.stream.centos.org/9-stream/BaseOS/x86_64/iso/CentOS-Stream-9-latest-x86_64-boot.iso"
        );
        assert!(list_centos_stream_official_download_candidates("../private.iso").is_err());
        assert!(list_centos_stream_official_download_candidates(
            "9-stream/AppStream/x86_64/iso/unexpected.iso"
        )
        .is_err());
    }

    #[test]
    fn oracle_linux_resolver_rejects_non_official_release_and_arch_paths() {
        let candidates = list_oracle_linux_official_download_candidates(
            "OL10/u7/x86_64/OracleLinux-R10-U7-x86_64-dvd.iso",
        )
        .unwrap();
        assert_eq!(
            candidates[0].url,
            "https://yum.oracle.com/ISOS/OracleLinux/OL10/u7/x86_64/OracleLinux-R10-U7-x86_64-dvd.iso"
        );
        assert!(list_oracle_linux_official_download_candidates("../private.iso").is_err());
        assert!(list_oracle_linux_official_download_candidates("OL8/u9/x86_64/old.iso").is_err());
    }

    #[test]
    fn rocky_resolver_builds_only_official_versioned_iso_paths() {
        let candidates =
            list_rocky_official_download_candidates("10/isos/x86_64/Rocky-10.1-x86_64-minimal.iso")
                .unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].url,
            "https://download.rockylinux.org/pub/rocky/10/isos/x86_64/Rocky-10.1-x86_64-minimal.iso"
        );
        assert!(list_rocky_official_download_candidates("../private.iso").is_err());
        assert!(list_rocky_official_download_candidates("8/isos/x86_64/old.iso").is_err());
    }

    #[test]
    fn almalinux_resolver_rejects_unsafe_and_unexpected_release_paths() {
        assert!(list_almalinux_official_download_candidates("../private.iso").is_err());
        assert!(list_almalinux_official_download_candidates("8/isos/x86_64/old.iso").is_err());
    }

    #[test]
    fn publisher_mirror_directory_parser_filters_bsd_roots_and_exact_iso_paths() {
        let html = r#"
            <a href="https://mirror.example/pub/OpenBSD/">OpenBSD good</a>
            <a href="http://plain.example/pub/OpenBSD/">OpenBSD plain</a>
            <a href="https://mirror.example/pub/NetBSD/">NetBSD good</a>
            <a href="https://mirror.example/pub/pkgsrc/">wrong tree</a>
        "#;
        let openbsd = parse_publisher_mirror_directory(
            html,
            "https://www.openbsd.org/ftp.html",
            "7.9/amd64/install79.iso",
            &["/pub/openbsd/"],
            "OpenBSD",
        )
        .unwrap();
        assert_eq!(openbsd.len(), 1);
        assert_eq!(
            openbsd[0].url,
            "https://mirror.example/pub/OpenBSD/7.9/amd64/install79.iso"
        );
        let netbsd = parse_publisher_mirror_directory(
            html,
            "https://www.netbsd.org/mirrors/",
            "images/10.1/amd64/NetBSD-10.1-amd64.iso",
            &["/pub/netbsd/"],
            "NetBSD",
        )
        .unwrap();
        assert_eq!(netbsd.len(), 1);
        assert!(netbsd[0]
            .url
            .ends_with("/pub/NetBSD/images/10.1/amd64/NetBSD-10.1-amd64.iso"));
    }

    #[test]
    fn opensuse_mirrorlist_parser_keeps_only_exact_https_iso_mirrors() {
        let html = r#"
            <a href="https://mirror.example/tumbleweed/iso/example.iso">good</a>
            <a href="http://plain.example/tumbleweed/iso/example.iso">plain</a>
            <a href="https://mirror.example/tumbleweed/iso/other.iso">wrong file</a>
            <a href="https://user:pass@mirror.example/tumbleweed/iso/example.iso">credentials</a>
            <a href="https://mirror.example/tumbleweed/iso/example.iso">duplicate</a>
        "#;
        let candidates = parse_opensuse_mirrorlist(html, "tumbleweed/iso/example.iso").unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].provider, "openSUSE MirrorCache");
        assert_eq!(
            candidates[0].url,
            "https://mirror.example/tumbleweed/iso/example.iso"
        );
        assert!(parse_opensuse_mirrorlist(
            "<a href='http://plain.example/file.iso'>x</a>",
            "file.iso"
        )
        .is_err());
    }

    #[test]
    fn opensuse_resolver_rejects_unsafe_artifact_paths_before_network_access() {
        assert!(list_opensuse_official_download_candidates("../private.iso").is_err());
        assert!(list_opensuse_official_download_candidates("/absolute.iso").is_err());
        assert!(list_opensuse_official_download_candidates("folder\\\\image.iso").is_err());
    }

    #[test]
    fn ubuntu_official_iso_resolver_uses_publisher_hosts_and_rejects_unknown_projects() {
        let ubuntu = list_ubuntu_official_download_candidates(
            "ubuntu",
            "24.04.3",
            "ubuntu-24.04.3-desktop-amd64.iso",
        )
        .unwrap();
        assert_eq!(
            ubuntu[0].url,
            "https://releases.ubuntu.com/24.04.3/ubuntu-24.04.3-desktop-amd64.iso"
        );

        let lubuntu = list_ubuntu_official_download_candidates(
            "lubuntu",
            "24.04.3",
            "lubuntu-24.04.3-desktop-amd64.iso",
        )
        .unwrap();
        assert!(lubuntu[0]
            .url
            .starts_with("https://cdimage.ubuntu.com/lubuntu/releases/24.04.3/release/"));
        assert!(list_ubuntu_official_download_candidates("mint", "24.04", "mint.iso").is_err());
        assert!(
            list_ubuntu_official_download_candidates("ubuntu", "../24.04", "ubuntu.iso").is_err()
        );
        assert!(
            list_ubuntu_official_download_candidates("ubuntu", "24.04", "../ubuntu.iso").is_err()
        );
    }

    #[test]
    fn fedora_path_inputs_reject_traversal_and_unsupported_architectures() {
        // Input validation is performed before any DNS or network access.
        assert!(list_fedora_official_download_candidates(
            "..",
            "Workstation",
            "x86_64",
            "fedora.iso"
        )
        .is_err());
        assert!(list_fedora_official_download_candidates(
            "45",
            "Workstation",
            "i686",
            "fedora.iso"
        )
        .is_err());
        assert!(list_fedora_official_download_candidates(
            "45",
            "Workstation",
            "x86_64",
            "../fedora.iso"
        )
        .is_err());
    }

    #[test]
    fn debian_candidate_uses_only_the_official_cdn_and_safe_relative_paths() {
        let candidates = list_debian_official_download_candidates(
            "current/amd64/iso-cd/debian-stable-amd64-netinst.iso",
        )
        .unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].provider, "Debian CloudFront");
        assert!(candidates[0]
            .url
            .starts_with("https://cloudfront.debian.net/debian-cd/current/amd64/iso-cd/"));
        assert!(list_debian_official_download_candidates("../private.iso").is_err());
        assert!(list_debian_official_download_candidates("/absolute.iso").is_err());
        assert!(list_debian_official_download_candidates("folder\\\\image.iso").is_err());
    }

    #[test]
    fn distro_center_downloads_are_limited_to_official_checksum_adapters() {
        let gate = distro_center_release_gate();
        assert!(gate.downloads_enabled);
        assert_eq!(gate.download_enabled_systems, 12);
        assert_eq!(gate.required_systems, 40);
        for id in [
            "debian", "arch", "ubuntu", "lubuntu", "kubuntu", "xubuntu", "fedora", "opensuse",
            "centos-stream", "antix", "rocky-linux", "almalinux",
        ] {
            assert!(gate
                .enabled_distribution_ids
                .iter()
                .any(|enabled| enabled == id));
        }
        assert!(!gate.enabled_distribution_ids.iter().any(|enabled| enabled == "oracle-linux"));
        assert!(ensure_distro_center_downloads_enabled("kali", "kali.iso", None).is_err());
        assert!(ensure_distro_center_downloads_enabled("fedora", "fedora.iso", None).is_err());
        assert!(ensure_distro_center_downloads_enabled("opensuse", "opensuse.iso", None).is_err());
        assert!(ensure_distro_center_downloads_enabled("debian", "debian.iso", None).is_err());
    }

    #[test]
    fn every_catalogue_entry_has_an_explicit_official_source_policy() {
        let catalogue = crate::catalog::load_distributions().expect("catalogue should load");
        assert_eq!(
            catalogue.len(),
            40,
            "Distro Center must cover all 40 entries"
        );

        for distribution in &catalogue {
            let support = mirror_support_for_distribution(&distribution.id)
                .unwrap_or_else(|_| panic!("missing source policy for {}", distribution.id));
            assert_eq!(support.distribution_id, distribution.id);
            assert!(support.source_url.starts_with("https://"));
            assert!(!support.label.trim().is_empty());
            assert!(!support.description.trim().is_empty());
        }

        assert!(mirror_support_for_distribution("arch").unwrap().selectable);
        assert!(
            !mirror_support_for_distribution("windows-11-x64")
                .unwrap()
                .selectable
        );
    }

    #[test]
    fn arch_mirror_parser_keeps_only_healthy_https_and_prefers_chile() {
        let document = serde_json::json!({"urls": [
            {"url":"https://far.example/arch/", "protocol":"https", "active":true, "country":"Farland", "completion_pct":100.0, "delay":120.0, "score":2.0},
            {"url":"https://cl.example/arch/", "protocol":"https", "active":true, "country":"Chile", "completion_pct":100.0, "delay":300.5, "score":4.0},
            {"url":"http://plain.example/arch/", "protocol":"http", "active":true, "country":"Chile", "completion_pct":100.0, "delay":1.0, "score":1.0},
            {"url":"https://stale.example/arch/", "protocol":"https", "active":true, "country":"Chile", "completion_pct":100.0, "delay":25000.0, "score":1.0},
            {"url":"https://weak.example/arch/", "protocol":"https", "active":true, "country":"Chile", "completion_pct":80.0, "delay":1.0, "score":1.0}
        ]});
        let mirrors = parse_arch_mirror_status(&document).unwrap();
        assert_eq!(mirrors.len(), 2);
        assert_eq!(mirrors[0].country, "Chile");
        assert_eq!(mirrors[0].delay_seconds, 301);
        assert_eq!(mirrors[1].country, "Farland");
    }

    #[test]
    fn kali_redirect_parser_uses_only_https_candidates_advertised_for_exact_iso() {
        let path = "kali-2026.3/kali-linux-2026.3-installer-amd64.iso";
        let links = vec![
            "<https://mirror.example/kali-images/kali-2026.3/kali-linux-2026.3-installer-amd64.iso>; rel=duplicate; pri=2; geo=gb, <http://plain.example/kali-images/kali-2026.3/kali-linux-2026.3-installer-amd64.iso>; rel=duplicate; pri=1; geo=cl".to_owned(),
            "<https://kali.download/base-images/kali-2026.3/kali-linux-2026.3-installer-amd64.iso>; rel=duplicate; pri=1; geo=global".to_owned(),
        ];
        let candidates = parse_kali_redirect_candidates(
            Some("https://redirect.example/kali-images/kali-2026.3/kali-linux-2026.3-installer-amd64.iso"),
            &links,
            path,
        )
        .unwrap();
        assert_eq!(candidates.len(), 3);
        assert_eq!(candidates[0].priority, 0);
        assert_eq!(candidates[1].priority, 1);
        assert!(candidates
            .iter()
            .all(|candidate| candidate.url.starts_with("https://")));
        assert!(!candidates
            .iter()
            .any(|candidate| candidate.url.contains("plain.example")));
    }

    #[test]
    fn kali_redirect_parser_rejects_wrong_artifact_and_untrusted_protocols() {
        let path = "kali-2026.3/kali-linux-2026.3-installer-amd64.iso";
        let links = vec![
            "<https://mirror.example/other.iso>; rel=duplicate; pri=1".to_owned(),
            "<http://mirror.example/kali-2026.3/kali-linux-2026.3-installer-amd64.iso>; rel=duplicate; pri=2".to_owned(),
        ];
        assert!(parse_kali_redirect_candidates(None, &links, path).is_err());
        assert!(list_kali_official_download_candidates("../private.iso").is_err());
        assert!(list_kali_official_download_candidates("/absolute.iso").is_err());
    }

    #[test]
    fn arch_mirror_parser_rejects_missing_or_empty_lists() {
        assert!(parse_arch_mirror_status(&serde_json::json!({})).is_err());
        assert!(parse_arch_mirror_status(&serde_json::json!({
            "urls": []
        }))
        .is_err());
    }

    #[test]
    fn rocky_checksum_adapter_builds_only_supported_official_manifest_paths() {
        assert_eq!(
            rocky_checksum_url_for_iso("Rocky-10.1-x86_64-minimal.iso").unwrap().0,
            "https://download.rockylinux.org/pub/rocky/10/isos/x86_64/CHECKSUM"
        );
        assert_eq!(
            rocky_checksum_url_for_iso("Rocky-9-latest-x86_64-dvd.iso").unwrap().0,
            "https://download.rockylinux.org/pub/rocky/9/isos/x86_64/CHECKSUM"
        );
        assert!(rocky_checksum_url_for_iso("../Rocky-10-x86_64-dvd.iso").is_err());
        assert!(rocky_checksum_url_for_iso("Rocky-8.10-x86_64-dvd.iso").is_err());
        assert!(rocky_checksum_url_for_iso("Rocky-10.1-aarch64-dvd.iso").is_err());
        assert!(rocky_checksum_url_for_iso("Rocky-10.1-x86_64-other.iso").is_err());
        assert!(rocky_checksum_url_for_iso("Rocky-10.1-odd-x86_64-dvd.iso").is_err());
    }

    #[test]
    fn almalinux_checksum_adapter_builds_only_supported_official_manifest_paths() {
        assert_eq!(
            almalinux_checksum_url_for_iso("AlmaLinux-10.2-x86_64-dvd.iso").unwrap().0,
            "https://repo.almalinux.org/almalinux/10/isos/x86_64/CHECKSUM"
        );
        assert_eq!(
            almalinux_checksum_url_for_iso("AlmaLinux-9.6-x86_64-minimal.iso").unwrap().0,
            "https://repo.almalinux.org/almalinux/9/isos/x86_64/CHECKSUM"
        );
        assert!(almalinux_checksum_url_for_iso("../AlmaLinux-10-x86_64-dvd.iso").is_err());
        assert!(almalinux_checksum_url_for_iso("AlmaLinux-8.10-x86_64-dvd.iso").is_err());
        assert!(almalinux_checksum_url_for_iso("AlmaLinux-10.2-aarch64-dvd.iso").is_err());
        assert!(almalinux_checksum_url_for_iso("AlmaLinux-10.2-x86_64-other.iso").is_err());
        assert!(almalinux_checksum_url_for_iso("AlmaLinux-10.2-odd-x86_64-dvd.iso").is_err());
    }

    #[test]
    fn oracle_linux_checksum_adapter_builds_only_supported_signed_manifest_paths() {
        assert_eq!(
            oracle_linux_checksum_url_for_iso("OracleLinux-R10-U2-x86_64-dvd.iso").unwrap().0,
            "https://linux.oracle.com/security/gpg/checksum/OracleLinux-R10-U2-Server-x86_64.checksum"
        );
        assert_eq!(
            oracle_linux_checksum_url_for_iso("OracleLinux-R9-U9-x86_64-boot-uek.iso").unwrap().0,
            "https://linux.oracle.com/security/gpg/checksum/OracleLinux-R9-U9-Server-x86_64.checksum"
        );
        assert!(oracle_linux_checksum_url_for_iso("../OracleLinux-R10-U2-x86_64-dvd.iso").is_err());
        assert!(oracle_linux_checksum_url_for_iso("OracleLinux-R8-U10-x86_64-dvd.iso").is_err());
        assert!(oracle_linux_checksum_url_for_iso("OracleLinux-R10-U2-aarch64-dvd.iso").is_err());
        assert!(oracle_linux_checksum_url_for_iso("OracleLinux-R10-U2-x86_64-src.iso").is_err());
        assert!(oracle_linux_checksum_url_for_iso("OracleLinux-R10-2-x86_64-dvd.iso").is_err());
    }

    #[test]
    fn centos_stream_checksum_adapter_builds_only_supported_sidecar_paths() {
        assert_eq!(
            centos_stream_checksum_url_for_iso("CentOS-Stream-10-latest-x86_64-boot.iso").unwrap().0,
            "https://mirror.stream.centos.org/10-stream/BaseOS/x86_64/iso/CentOS-Stream-10-latest-x86_64-boot.iso.SHA256SUM"
        );
        assert_eq!(
            centos_stream_checksum_url_for_iso("CentOS-Stream-9-20261118.0-x86_64-dvd1.iso").unwrap().0,
            "https://mirror.stream.centos.org/9-stream/BaseOS/x86_64/iso/CentOS-Stream-9-20261118.0-x86_64-dvd1.iso.SHA256SUM"
        );
        assert!(centos_stream_checksum_url_for_iso("../CentOS-Stream-10-latest-x86_64-boot.iso").is_err());
        assert!(centos_stream_checksum_url_for_iso("CentOS-Stream-8-latest-x86_64-boot.iso").is_err());
        assert!(centos_stream_checksum_url_for_iso("CentOS-Stream-10-latest-aarch64-boot.iso").is_err());
        assert!(centos_stream_checksum_url_for_iso("CentOS-Stream-10-latest-x86_64-other.iso").is_err());
        assert!(centos_stream_checksum_url_for_iso("CentOS-Stream-10-2026-x86_64-boot.iso").is_err());
    }

    #[test]
    fn antix_sha256_parser_accepts_exact_publisher_hash_and_rejects_conflicts() {
        let html = r#"
            <section>
              <h3>antiX 26.1</h3>
              <p>293381af0b7c48399abe015da8f67d41fd34372b73c4aafcf53747fbaed22371
                antiX-26.1_x64-full.iso</p>
              <p>6ec6f6522c69b4411d32f61e853709cdaf94df709c264b2a4d7de9724ce57fbc
                antiX-26.1_386-full.iso</p>
            </section>
        "#;
        assert_eq!(
            parse_antix_sha256_html(html, "antiX-26.1_x64-full.iso").unwrap(),
            "293381af0b7c48399abe015da8f67d41fd34372b73c4aafcf53747fbaed22371"
        );
        assert!(parse_antix_sha256_html(html, "antiX-26.1_x64-core.iso").is_err());
        assert!(parse_antix_sha256_html(html, "../antiX.iso").is_err());

        let conflict = format!(
            "{html}<p>{} antiX-26.1_x64-full.iso</p>",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        );
        assert!(parse_antix_sha256_html(&conflict, "antiX-26.1_x64-full.iso").is_err());
    }

    #[test]
    fn exposes_only_explicitly_allowlisted_release_sources() {
        let sources = supported_release_sources();
        assert_eq!(sources.len(), 13);
        for expected in [
            "debian", "arch", "ubuntu", "lubuntu", "kubuntu", "xubuntu", "fedora", "opensuse",
        ] {
            assert!(
                sources.iter().any(|(id, _)| id == expected),
                "missing {expected}"
            );
        }
        assert!(!sources.iter().any(|(id, _)| id == "linuxmint"));
    }

    #[test]
    fn rejects_paths_and_non_iso_names() {
        for value in [
            "",
            "../debian.iso",
            "folder/debian.iso",
            "debian.zip",
            "debian.iso/other.iso",
            "debian image.iso",
        ] {
            assert!(!safe_iso_name(value), "accepted {value}");
        }
        assert!(safe_iso_name("debian-13.7.0-amd64-netinst.iso"));
        assert!(safe_iso_name("archlinux-2026.11.01-x86_64.iso"));
    }

    #[test]
    fn rejects_unconfigured_distribution_without_network_access() {
        let error = lookup_official_release_checksum("linuxmint", "linuxmint.iso").unwrap_err();
        assert!(error.contains("todavía no tiene"));
    }

    #[test]
    fn publisher_iso_link_parser_requires_exact_https_filename_and_allowed_host() {
        let html = r#"
            <a href="https://downloads.getsol.us/isos/Solus-Budgie.iso">good</a>
            <a href="http://downloads.getsol.us/isos/Solus-Budgie.iso">plain</a>
            <a href="https://evil.example/isos/Solus-Budgie.iso">wrong host</a>
            <a href="https://downloads.getsol.us/isos/Other.iso">wrong file</a>
            <a href="https://downloads.getsol.us/isos/Solus-Budgie.iso?token=x">query</a>
        "#;
        let candidates = parse_publisher_iso_links(
            html,
            "https://getsol.us/download/",
            "Solus-Budgie.iso",
            &["downloads.getsol.us"],
        )
        .unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].url,
            "https://downloads.getsol.us/isos/Solus-Budgie.iso"
        );
        assert!(parse_publisher_iso_links(
            html,
            "https://getsol.us/download/",
            "../Solus-Budgie.iso",
            &["downloads.getsol.us"],
        )
        .is_err());
    }

    #[test]
    fn publisher_resolvers_reject_invalid_names_before_network_access() {
        assert!(list_linux_lite_official_download_candidates("../image.iso").is_err());
        assert!(list_solus_official_download_candidates("other.iso").is_err());
        assert!(list_antix_official_download_candidates("image.iso").is_err());
    }

    #[test]
    fn slackware_resolver_is_pinned_to_publisher_iso_tree() {
        let candidates = list_slackware_official_download_candidates(
            "slackware-iso/slackware64-15.0-iso/slackware64-15.0-install-dvd.iso",
        )
        .unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].url,
            "https://mirrors.slackware.com/slackware/slackware-iso/slackware64-15.0-iso/slackware64-15.0-install-dvd.iso"
        );
        assert!(list_slackware_official_download_candidates("../image.iso").is_err());
        assert!(list_slackware_official_download_candidates(
            "slackware64-current/untrusted.iso"
        )
        .is_err());
    }

    #[test]
    fn reactos_resolver_is_pinned_to_official_nightly_iso_archive() {
        let candidates = list_reactos_official_download_candidates(
            "bootcd/reactos-bootcd-20261118-r12345-gabc1234.iso",
        )
        .unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].url,
            "https://iso.reactos.org/bootcd/reactos-bootcd-20261118-r12345-gabc1234.iso"
        );
        assert!(candidates[0].region.contains("nocturno"));
        assert!(list_reactos_official_download_candidates("../bootcd/image.iso").is_err());
        assert!(list_reactos_official_download_candidates(
            "other/image.iso"
        )
        .is_err());
        assert!(list_reactos_official_download_candidates(
            "bootcd/subdir/image.iso"
        )
        .is_err());
    }


    #[test]
    fn windows_distribution_ids_use_the_correct_microsoft_download_pages() {
        let expected = [
            (
                "windows-11-x64",
                "https://www.microsoft.com/software-download/windows11",
            ),
            (
                "windows-11-arm64",
                "https://www.microsoft.com/software-download/windows11arm64",
            ),
            (
                "windows-10-home",
                "https://www.microsoft.com/en-us/software-download/windows10ISO",
            ),
            (
                "windows-10-pro",
                "https://www.microsoft.com/en-us/software-download/windows10ISO",
            ),
            (
                "windows-10-ltsc",
                "https://www.microsoft.com/en-us/evalcenter/download-windows-10-enterprise",
            ),
        ];
        for (distribution_id, expected_url) in expected {
            let support = mirror_support_for_distribution(distribution_id).unwrap();
            assert_eq!(support.source_url, expected_url, "{distribution_id}");
            assert!(!support.selectable);
        }
    }

    #[test]
    fn reactos_policy_marks_the_buildbot_channel_as_nightly() {
        let support = mirror_support_for_distribution("reactos").unwrap();
        assert_eq!(support.mode, "official_nightly_iso_archive");
        assert_eq!(
            support.source_url,
            "https://reactos.org/getbuilds/index.php?lang=en"
        );
        assert!(support.description.contains("nocturnos"));
    }


}
