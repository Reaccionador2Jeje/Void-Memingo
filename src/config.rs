use serde::Deserialize;
use std::fs;
use std::path::Path;

/// Configuración principal de un proyecto Void Memingo.
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct VoidConfig {
    pub project: ProjectSection,
    #[serde(default)]
    pub build: BuildSection,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct ProjectSection {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct BuildSection {
    #[serde(default = "default_target")]
    pub target: String,
}

impl Default for BuildSection {
    fn default() -> Self {
        Self { target: default_target() }
    }
}

fn default_target() -> String {
    "x86_64-unknown-none".to_string()
}

/// Lee y analiza un archivo void.toml.
pub fn load(path: &Path) -> Result<VoidConfig, String> {
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("Cannot read {}: {}", path.display(), error))?;

    let config: VoidConfig = toml::from_str(&contents)
        .map_err(|error| format!("Invalid configuration: {}", error))?;

    validate(&config)?;
    Ok(config)
}

/// Comprueba los campos que Void necesita para trabajar.
fn validate(config: &VoidConfig) -> Result<(), String> {
    if config.project.name.trim().is_empty() {
        return Err("The project name cannot be empty.".into());
    }

    if config.project.version.trim().is_empty() {
        return Err("The project version cannot be empty.".into());
    }

    if config.build.target.trim().is_empty() {
        return Err("The build target cannot be empty.".into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_configuration() {
        let source = r#"
            [project]
            name = "TestOS"
            version = "0.1.0"

            [build]
            target = "x86_64-unknown-none"
        "#;

        let config: VoidConfig = toml::from_str(source).unwrap();
        assert_eq!(config.project.name, "TestOS");
        assert_eq!(config.build.target, "x86_64-unknown-none");
    }

    #[test]
    fn applies_default_build_target() {
        let source = r#"
            [project]
            name = "TestOS"
            version = "0.1.0"
        "#;

        let config: VoidConfig = toml::from_str(source).unwrap();
        assert_eq!(config.build.target, "x86_64-unknown-none");
    }

    #[test]
    fn rejects_invalid_toml() {
        let source = "[project\nname = broken";
        assert!(toml::from_str::<VoidConfig>(source).is_err());
    }
}
