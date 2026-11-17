use serde::Serialize;
use std::{
    io::Read,
    net::ToSocketAddrs,
    time::Duration,
};

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
        && value == std::path::Path::new(value)
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
        .map(|source| (source.distribution_id.to_owned(), source.release_label.to_owned()))
        .collect()
}

pub fn lookup_official_release_checksum(
    distribution_id: &str,
    file_name: &str,
) -> Result<OfficialReleaseChecksum, String> {
    if !safe_iso_name(file_name) {
        return Err("Indica el nombre exacto de una ISO, sin rutas ni caracteres especiales.".into());
    }

    let source = SOURCES
        .iter()
        .find(|source| source.distribution_id == distribution_id)
        .ok_or_else(|| "Esta distribución todavía no tiene un adaptador oficial de checksums.".to_string())?;

    let url = reqwest::Url::parse(source.manifest_url)
        .map_err(|error| format!("URL del adaptador inválida: {error}"))?;
    if url.scheme() != "https" || url.host_str() != Some(source.allowed_host) {
        return Err("La fuente oficial no supera la validación HTTPS del catálogo.".into());
    }

    let port = url.port_or_known_default().ok_or_else(|| "No se pudo determinar el puerto HTTPS.".to_string())?;
    let addresses = (source.allowed_host, port)
        .to_socket_addrs()
        .map_err(|error| format!("No se pudo resolver la fuente oficial: {error}"))?
        .collect::<Vec<_>>();
    if addresses.is_empty() || addresses.iter().any(|address| !crate::download::is_safe_ip(address.ip())) {
        return Err("La fuente oficial resolvió a una dirección no pública; se canceló la consulta.".into());
    }

    let client = reqwest::blocking::Client::builder()
        .resolve_to_addrs(source.allowed_host, &addresses)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(15))
        .user_agent("VoidBoot-ReleaseMetadata/0.1")
        .build()
        .map_err(|error| format!("No se pudo preparar la consulta HTTPS: {error}"))?;

    let mut response = client
        .get(source.manifest_url)
        .send()
        .map_err(|error| format!("No se pudo consultar el manifiesto oficial: {error}"))?;
    if !response.status().is_success() || response.url().as_str() != source.manifest_url {
        return Err(format!(
            "La fuente oficial respondió con {} o intentó redirigir; no se siguieron redirecciones.",
            response.status()
        ));
    }
    if response.content_length().is_some_and(|length| length > MAX_MANIFEST_BYTES) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_only_explicitly_allowlisted_release_sources() {
        let sources = supported_release_sources();
        assert_eq!(sources.len(), 5);
        for expected in ["debian", "arch", "ubuntu", "fedora", "opensuse"] {
            assert!(sources.iter().any(|(id, _)| id == expected), "missing {expected}");
        }
        assert!(!sources.iter().any(|(id, _)| id == "linuxmint"));
    }

    #[test]
    fn rejects_paths_and_non_iso_names() {
        for value in ["", "../debian.iso", "folder/debian.iso", "debian.zip", "debian.iso/other.iso", "debian image.iso"] {
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
