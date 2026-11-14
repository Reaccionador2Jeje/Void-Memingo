use serde::{Deserialize, Serialize};

fn default_kind() -> String {
    "linux_distribution".to_owned()
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Distribution {
    pub id: String,
    #[serde(default = "default_kind")]
    pub kind: String,
    pub name: String,
    pub description: String,
    pub family: String,
    pub project_url: String,
    pub download_page: String,
    pub architectures: Vec<String>,
    pub editions: Vec<String>,
    pub release_model: String,
    pub formats: Vec<String>,
    pub verification: String,
    pub installation: InstallationMetadata,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct InstallationMetadata {
    pub delivery: String,
    pub requires_full_image: bool,
    pub boot_profile_required: bool,
    pub note: String,
}

const DISTRIBUTIONS_JSON: &str = include_str!("../catalog/distributions.json");

pub fn load_distributions() -> Result<Vec<Distribution>, serde_json::Error> {
    let entries: Vec<Distribution> = serde_json::from_str(DISTRIBUTIONS_JSON)?;
    let mut ids = std::collections::HashSet::new();

    for entry in &entries {
        let safe_linux_entry = entry.kind == "linux_distribution"
            && entry.installation.delivery == "direct_iso_to_void_data_partition"
            && entry.installation.requires_full_image
            && entry.installation.boot_profile_required;
        let safe_external_os_entry = entry.kind == "operating_system"
            && entry.installation.delivery == "official_media_page_only"
            && entry.installation.requires_full_image
            && !entry.installation.boot_profile_required;

        if !entry.project_url.starts_with("https://")
            || !entry.download_page.starts_with("https://")
            || entry.id.trim().is_empty()
            || !ids.insert(entry.id.to_ascii_lowercase())
            || entry.name.trim().is_empty()
            || entry.description.trim().is_empty()
            || entry.family.trim().is_empty()
            || entry.architectures.is_empty()
            || entry.editions.is_empty()
            || entry.formats.is_empty()
            || entry.verification.trim().is_empty()
            || !(safe_linux_entry || safe_external_os_entry)
            || entry.installation.note.trim().is_empty()
        {
            return Err(<serde_json::Error as serde::de::Error>::custom(
                "catalog entries require unique identity, HTTPS sources, architectures, editions, formats, verification and a supported safe delivery mode",
            ));
        }
    }
    Ok(entries)
}

pub fn filter_distributions<'a>(
    entries: &'a [Distribution],
    family: Option<&str>,
    architecture: Option<&str>,
) -> Vec<&'a Distribution> {
    entries
        .iter()
        .filter(|entry| {
            family.is_none_or(|value| entry.family.eq_ignore_ascii_case(value))
                && architecture.is_none_or(|value| {
                    entry
                        .architectures
                        .iter()
                        .any(|arch| arch.eq_ignore_ascii_case(value))
                })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_catalog_has_unique_ids_and_complete_https_metadata() {
        let entries = load_distributions().expect("bundled catalog should be valid");
        assert!(entries.len() >= 20);
        let mut ids = std::collections::HashSet::new();
        for entry in entries {
            assert!(ids.insert(entry.id), "catalog IDs must be unique");
            assert!(entry.project_url.starts_with("https://"));
            assert!(entry.download_page.starts_with("https://"));
            assert!(!entry.architectures.is_empty());
            assert!(!entry.editions.is_empty());
            assert!(!entry.formats.is_empty());
            assert!(!entry.verification.is_empty());
            assert!(!entry.installation.note.is_empty());
        }
    }

    #[test]
    fn filters_by_family_and_architecture() {
        let entries = load_distributions().expect("catalog should load");
        let results = filter_distributions(&entries, Some("debian"), Some("x86_64"));
        assert!(results.iter().any(|entry| entry.id == "debian"));
        assert!(results.iter().any(|entry| entry.id == "kali"));
        assert!(!results.iter().any(|entry| entry.id == "fedora"));
    }

    #[test]
    fn windows_entries_are_official_media_only_and_not_boot_compatible() {
        let entries = load_distributions().expect("catalog should load");
        let windows = filter_distributions(&entries, Some("windows"), None);
        assert_eq!(windows.len(), 2);
        for entry in windows {
            assert_eq!(entry.kind, "operating_system");
            assert_eq!(entry.installation.delivery, "official_media_page_only");
            assert!(!entry.installation.boot_profile_required);
            assert!(entry.download_page.starts_with("https://www.microsoft.com/"));
        }
    }

    #[test]
    fn catalog_distinguishes_linux_and_external_operating_systems() {
        let entries = load_distributions().expect("catalog should load");
        assert!(entries.iter().any(|entry| entry.kind == "linux_distribution"));
        assert!(entries.iter().any(|entry| entry.kind == "operating_system"));
        assert!(entries.iter().filter(|entry| entry.kind == "linux_distribution")
            .all(|entry| entry.installation.delivery == "direct_iso_to_void_data_partition"));
        assert!(entries.iter().filter(|entry| entry.kind == "operating_system")
            .all(|entry| entry.installation.delivery == "official_media_page_only"
                && !entry.installation.boot_profile_required));
    }
}
