use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Distribution {
    pub id: String,
    pub name: String,
    pub description: String,
    pub project_url: String,
    pub download_page: String,
    pub formats: Vec<String>,
    pub verification: String,
}

const DISTRIBUTIONS_JSON: &str = include_str!("../catalog/distributions.json");

pub fn load_distributions() -> Result<Vec<Distribution>, serde_json::Error> {
    let entries: Vec<Distribution> = serde_json::from_str(DISTRIBUTIONS_JSON)?;
    for entry in &entries {
        if !entry.project_url.starts_with("https://")
            || !entry.download_page.starts_with("https://")
            || entry.id.is_empty()
            || entry.name.is_empty()
        {
            return Err(<serde_json::Error as serde::de::Error>::custom(
                "catalog entries require an ID, name and HTTPS project/download URLs",
            ));
        }
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_catalog_has_unique_ids_and_https_sources() {
        let entries = load_distributions().expect("bundled distribution catalog should be valid");
        assert!(!entries.is_empty());
        let mut ids = std::collections::HashSet::new();
        for entry in entries {
            assert!(ids.insert(entry.id), "distribution IDs must be unique");
            assert!(entry.download_page.starts_with("https://"));
            assert!(!entry.formats.is_empty());
            assert!(!entry.verification.is_empty());
        }
    }
}
