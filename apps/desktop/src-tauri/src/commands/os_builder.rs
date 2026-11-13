use serde::{Deserialize, Serialize};
use std::path::{Component, Path};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseCapabilities {
    pub id: &'static str,
    pub name: &'static str,
    pub backend: &'static str,
    pub backend_status: &'static str,
    pub releases: &'static [&'static str],
    pub architectures: &'static [&'static str],
    pub desktops: &'static [&'static str],
    pub boot_modes: &'static [&'static str],
    pub notes: &'static str,
}

pub fn base_capabilities() -> Vec<BaseCapabilities> {
    vec![
        BaseCapabilities {
            id: "debian",
            name: "Debian",
            backend: "live-build",
            backend_status: "planned",
            releases: &["stable", "testing"],
            architectures: &["amd64"],
            desktops: &["gnome", "kde-plasma", "xfce", "lxqt", "cinnamon", "mate", "none"],
            boot_modes: &["uefi", "bios"],
            notes: "Debian Live configuration, package lists and root filesystem overlay.",
        },
        BaseCapabilities {
            id: "ubuntu",
            name: "Ubuntu",
            backend: "ubuntu-live",
            backend_status: "planned",
            releases: &["24.04 LTS", "26.04 LTS"],
            architectures: &["amd64"],
            desktops: &["gnome", "kde-plasma", "xfce", "lxqt", "cinnamon", "mate", "none"],
            boot_modes: &["uefi", "bios"],
            notes: "Ubuntu live image configuration with casper integration; installer image editing is a separate workflow.",
        },
        BaseCapabilities {
            id: "arch",
            name: "Arch Linux",
            backend: "archiso",
            backend_status: "planned",
            releases: &["rolling"],
            architectures: &["x86_64"],
            desktops: &["gnome", "kde-plasma", "xfce", "lxqt", "cinnamon", "mate", "none"],
            boot_modes: &["uefi", "bios"],
            notes: "archiso profile, package lists and airootfs overlay.",
        },
    ]
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct OsProject {
    pub project: ProjectIdentity,
    pub base: BaseConfig,
    #[serde(default)]
    pub packages: PackageConfig,
    #[serde(default)]
    pub filesystem: FilesystemConfig,
    #[serde(default)]
    pub desktop: DesktopConfig,
    #[serde(default)]
    pub boot: BootConfig,
    #[serde(default)]
    pub output: OutputConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProjectIdentity {
    pub name: String,
    pub id: String,
    pub version: String,
    #[serde(default)]
    pub publisher: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub iso_label: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BaseConfig {
    pub distribution: String,
    pub release: String,
    pub architecture: String,
    #[serde(default)]
    pub repositories: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PackageConfig {
    #[serde(default)]
    pub install: Vec<String>,
    #[serde(default)]
    pub remove: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct FilesystemConfig {
    #[serde(default)]
    pub overlay: String,
    #[serde(default)]
    pub services_enabled: Vec<String>,
    #[serde(default)]
    pub locale: String,
    #[serde(default)]
    pub timezone: String,
    #[serde(default)]
    pub hostname: String,
    #[serde(default)]
    pub storage_filesystem: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DesktopConfig {
    #[serde(default = "default_desktop")]
    pub environment: String,
    #[serde(default)]
    pub display_manager: String,
    #[serde(default)]
    pub applications: Vec<String>,
    #[serde(default)]
    pub theme: String,
    #[serde(default)]
    pub icon_theme: String,
    #[serde(default)]
    pub wallpaper: String,
}

fn default_desktop() -> String { "none".into() }

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct BootConfig {
    #[serde(default)]
    pub uefi: bool,
    #[serde(default)]
    pub bios: bool,
    #[serde(default)]
    pub live_session: bool,
    #[serde(default)]
    pub installer: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct OutputConfig {
    #[serde(default)]
    pub iso_name: String,
    #[serde(default)]
    pub directory: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationReport {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

fn safe_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 48
        && value.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && value.as_bytes()[0].is_ascii_lowercase()
}

fn safe_package_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|b| b.is_ascii_alphanumeric() || b"@+_.:-".contains(&b))
}

fn safe_project_relative_path(value: &str) -> bool {
    if value.is_empty() { return true; }
    let path = Path::new(value);
    !path.is_absolute()
        && path.components().all(|component| {
            matches!(component, Component::Normal(_) | Component::CurDir)
        })
}

pub fn validate_project(project: &OsProject) -> ValidationReport {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let base = project.base.distribution.to_ascii_lowercase();
    let capabilities = base_capabilities();
    let selected = capabilities.iter().find(|item| item.id == base);

    if project.project.name.trim().is_empty() {
        errors.push("Project name is required.".into());
    }
    if !safe_identifier(&project.project.id) {
        errors.push("Project id must use lowercase letters, numbers and hyphens.".into());
    }
    if project.project.version.trim().is_empty() {
        errors.push("Project version is required.".into());
    }

    match selected {
        None => errors.push("Base distribution must be debian, ubuntu or arch.".into()),
        Some(capability) => {
            if !capability.releases.contains(&project.base.release.as_str()) {
                errors.push(format!("Release '{}' is not listed for {}.", project.base.release, capability.name));
            }
            if !capability.architectures.contains(&project.base.architecture.as_str()) {
                errors.push(format!("Architecture '{}' is not listed for {}.", project.base.architecture, capability.name));
            }
            if !capability.desktops.contains(&project.desktop.environment.as_str()) {
                errors.push(format!("Desktop '{}' is not available for {}.", project.desktop.environment, capability.name));
            }
            if capability.backend_status != "ready" {
                warnings.push(format!("The {} backend is not implemented yet; this project can be validated but not built into an ISO.", capability.name));
            }
        }
    }

    if !project.packages.install.iter().all(|item| safe_package_name(item))
        || !project.packages.remove.iter().all(|item| safe_package_name(item))
    {
        errors.push("Package names contain unsupported characters.".into());
    }

    if !safe_project_relative_path(&project.filesystem.overlay) {
        errors.push("Filesystem overlay must be a safe path relative to the OS project.".into());
    }
    if !project.filesystem.hostname.is_empty()
        && (project.filesystem.hostname.len() > 63
            || !project.filesystem.hostname.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'))
    {
        errors.push("Hostname must contain only ASCII letters, numbers and hyphens (max 63 characters).".into());
    }
    if !project.project.iso_label.is_empty()
        && (project.project.iso_label.len() > 32
            || !project.project.iso_label.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-'))
    {
        errors.push("ISO label must be at most 32 characters and use letters, numbers, hyphens or underscores.".into());
    }
    if !project.boot.uefi && !project.boot.bios {
        warnings.push("Neither UEFI nor BIOS boot mode is selected.".into());
    }
    if project.filesystem.overlay.is_empty() {
        warnings.push("No root filesystem overlay is configured.".into());
    }
    if project.desktop.environment != "none" && project.desktop.display_manager.is_empty() {
        warnings.push("A desktop environment is selected but no display manager is specified.".into());
    }

    ValidationReport { valid: errors.is_empty(), errors, warnings }
}

#[tauri::command]
pub fn list_os_base_capabilities() -> Vec<BaseCapabilities> {
    base_capabilities()
}

#[tauri::command]
pub fn validate_os_project(project_toml: String) -> Result<ValidationReport, String> {
    let project: OsProject = toml::from_str(&project_toml)
        .map_err(|error| format!("Invalid void-os.toml: {error}"))?;
    Ok(validate_project(&project))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> OsProject {
        OsProject {
            project: ProjectIdentity {
                name: "Atdev OS".into(), id: "atdev-os".into(), version: "0.1.0".into(),
                publisher: "Atdev Dev".into(), description: String::new(), iso_label: "ATDEV_OS".into(),
            },
            base: BaseConfig {
                distribution: "debian".into(), release: "stable".into(), architecture: "amd64".into(),
                repositories: vec![],
            },
            packages: PackageConfig::default(),
            filesystem: FilesystemConfig::default(),
            desktop: DesktopConfig::default(),
            boot: BootConfig { uefi: true, bios: true, live_session: true, installer: false },
            output: OutputConfig::default(),
        }
    }

    #[test]
    fn exposes_only_the_three_product_bases() {
        let ids: Vec<_> = base_capabilities().iter().map(|item| item.id).collect();
        assert_eq!(ids, vec!["debian", "ubuntu", "arch"]);
    }

    #[test]
    fn accepts_valid_debian_project() {
        assert!(validate_project(&sample()).valid);
    }

    #[test]
    fn rejects_unknown_distribution() {
        let mut project = sample();
        project.base.distribution = "fedora".into();
        assert!(!validate_project(&project).valid);
    }

    #[test]
    fn rejects_unsafe_overlay_path() {
        let mut project = sample();
        project.filesystem.overlay = "../../etc".into();
        assert!(!validate_project(&project).valid);
    }
}
