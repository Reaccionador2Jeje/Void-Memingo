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
    SOURCES
        .iter()
        .map(|source| {
            (
                source.distribution_id.to_owned(),
                source.release_label.to_owned(),
            )
        })
        .collect()
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

    candidates.sort_by(|left, right| left.priority.cmp(&right.priority));
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
        if quote != b'"' && quote != b'\\'' {
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
            || !root_markers.iter().any(|marker| base_path.ends_with(&marker.to_ascii_lowercase()))
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
        return Err(format!("{provider} no anunció mirrors HTTPS válidos para esa ruta."));
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
        return Err(format!("La página de mirrors de {host} supera el límite permitido."));
    }
    let mut html = String::new();
    response
        .take(MAX_BSD_MIRROR_PAGE_BYTES + 1)
        .read_to_string(&mut html)
        .map_err(|error| format!("No se pudo leer la página de mirrors de {host}: {error}"))?;
    if html.len() as u64 > MAX_BSD_MIRROR_PAGE_BYTES {
        return Err(format!("La página de mirrors de {host} supera el límite permitido."));
    }
    Ok(html)
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
        if quote != b'"' && quote != b'\\'' {
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
    pub validated_systems: usize,
    pub required_systems: usize,
    pub message: String,
}

pub fn distro_center_release_gate() -> DistroCenterReleaseGate {
    DistroCenterReleaseGate {
        downloads_enabled: false,
        validated_systems: 0,
        required_systems: 40,
        message: "Las descargas están bloqueadas hasta completar y validar el soporte de los 40 sistemas del catálogo: descarga oficial, integridad, perfil de arranque y pruebas BIOS/UEFI cuando corresponda.".to_owned(),
    }
}

pub fn ensure_distro_center_downloads_enabled() -> Result<(), String> {
    let gate = distro_center_release_gate();
    if !gate.downloads_enabled || gate.validated_systems < gate.required_systems {
        return Err(gate.message);
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
        "mx-linux" => (
            "official_download_page",
            "Descarga oficial de MX Linux",
            "MX Linux publica sus propios enlaces de descarga. No se asume que los mirrors de paquetes Debian contengan sus ISOs.",
            "https://mxlinux.org/download-links/",
            false,
        ),
        "deepin" => (
            "official_download_page",
            "Descarga oficial de deepin",
            "Se utiliza el canal de descarga del proyecto; no se reutilizan mirrors Debian para imágenes ISO.",
            "https://www.deepin.org/en/download/",
            false,
        ),
        "antix" => (
            "official_download_page",
            "Descarga oficial de antiX",
            "Se utiliza la página oficial del proyecto hasta integrar su propio índice de imágenes y mirrors.",
            "https://antixlinux.com/download/",
            false,
        ),
        "linux-mint" => (
            "official_download_page",
            "Descarga oficial de Linux Mint",
            "Linux Mint mantiene su propio listado de mirrors de ISO; la selección automática aún no está integrada.",
            "https://linuxmint.com/download.php",
            false,
        ),
        "zorin-os" => (
            "official_download_page",
            "Descarga oficial de Zorin OS",
            "Se conserva la página oficial de descarga; no se asume compatibilidad con mirrors de Ubuntu para sus imágenes ISO.",
            "https://zorin.com/os/download/",
            false,
        ),
        "pop-os" => (
            "official_download_page",
            "Descarga oficial de Pop!_OS",
            "Se conserva el canal oficial de System76; no se reutilizan mirrors de paquetes Ubuntu como mirrors de ISO.",
            "https://system76.com/pop/download/",
            false,
        ),
        "elementary-os" => (
            "official_download_page",
            "Descarga oficial de elementary OS",
            "Se conserva el canal oficial del proyecto hasta integrar su distribución de imágenes.",
            "https://elementary.io/",
            false,
        ),
        "linux-lite" => (
            "official_download_page",
            "Descarga oficial de Linux Lite",
            "Se conserva la página oficial del proyecto; no se asume que mirrors de Ubuntu alojen sus imágenes ISO.",
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
            "La descarga oficial usa infraestructura GeoDNS/CDN; el servidor oficial selecciona una ruta cercana sin exponer una lista para elegir manualmente.",
            "https://download.freebsd.org/",
            false,
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
        "windows-11-x64" | "windows-11-arm64" | "windows-10-home" | "windows-10-pro" | "windows-10-ltsc" => (
            "official_single_source",
            "Descarga oficial de Microsoft",
            "Se utiliza la fuente oficial de Microsoft. No se ofrecen mirrors de terceros ni selección de servidores no autorizados.",
            "https://www.microsoft.com/software-download/",
            false,
        ),
        "reactos" => (
            "official_download_page",
            "Descarga oficial del proyecto",
            "Se conserva la página oficial de descarga; no hay un selector de mirrors verificado integrado.",
            "https://reactos.org/download/",
            false,
        ),
        "endeavouros" | "manjaro" | "nixos" | "void-linux" | "rocky-linux" | "almalinux" | "alpine" | "mageia" | "centos-stream" | "gentoo" | "slackware" | "solus" | "oracle-linux" => (
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

#[cfg(test)]
mod tests {
    use super::*;

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
        ).unwrap();
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
        ).unwrap();
        assert_eq!(netbsd.len(), 1);
        assert!(netbsd[0].url.ends_with("/pub/NetBSD/images/10.1/amd64/NetBSD-10.1-amd64.iso"));
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
        assert_eq!(candidates[0].url, "https://mirror.example/tumbleweed/iso/example.iso");
        assert!(parse_opensuse_mirrorlist("<a href='http://plain.example/file.iso'>x</a>", "file.iso").is_err());
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
    fn distro_center_downloads_stay_locked_until_every_system_is_validated() {
        let gate = distro_center_release_gate();
        assert!(!gate.downloads_enabled);
        assert_eq!(gate.validated_systems, 0);
        assert_eq!(gate.required_systems, 40);
        assert!(ensure_distro_center_downloads_enabled().is_err());
    }

    #[test]
    fn mirror_support_is_defined_for_linux_bsd_and_windows_catalogue_entries() {
        for id in [
            "debian",
            "ubuntu",
            "fedora",
            "arch",
            "linux-mint",
            "zorin-os",
            "opensuse",
            "pop-os",
            "elementary-os",
            "kali",
            "endeavouros",
            "manjaro",
            "mx-linux",
            "nixos",
            "void-linux",
            "rocky-linux",
            "almalinux",
            "alpine",
            "mageia",
            "windows-11-x64",
            "windows-11-arm64",
            "freebsd",
            "openbsd",
            "netbsd",
            "reactos",
            "centos-stream",
            "gentoo",
            "slackware",
            "lubuntu",
            "kubuntu",
            "xubuntu",
            "linux-lite",
            "deepin",
            "solus",
            "kde-neon",
            "oracle-linux",
            "antix",
            "windows-10-home",
            "windows-10-pro",
            "windows-10-ltsc",
        ] {
            let support = mirror_support_for_distribution(id)
                .unwrap_or_else(|_| panic!("missing mirror support for {id}"));
            assert_eq!(support.distribution_id, id);
            assert!(support.source_url.starts_with("https://"));
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
        ).unwrap();
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
        assert!(parse_arch_mirror_status(&serde_json::json!({"urls": []})).is_err());
    }

    #[test]
    fn exposes_only_explicitly_allowlisted_release_sources() {
        let sources = supported_release_sources();
        assert_eq!(sources.len(), 5);
        for expected in ["debian", "arch", "ubuntu", "fedora", "opensuse"] {
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
}
