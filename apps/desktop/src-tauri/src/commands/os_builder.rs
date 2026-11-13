use serde::{Deserialize, Serialize};
use std::{fs, path::{Component, Path, PathBuf}};

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
            matches!(component, Component::Normal(_))
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


#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildPreparation {
    pub backend: String,
    pub build_directory: String,
    pub script_path: String,
    pub commands: Vec<String>,
    pub notes: Vec<String>,
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn copy_overlay_tree(source: &Path, destination: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(source)
        .map_err(|error| format!("Cannot inspect filesystem overlay: {error}"))?;
    if metadata.file_type().is_symlink() {
        return Err("Filesystem overlay cannot contain symbolic links.".into());
    }
    if metadata.is_dir() {
        fs::create_dir_all(destination)
            .map_err(|error| format!("Cannot create overlay directory: {error}"))?;
        for entry in fs::read_dir(source).map_err(|error| format!("Cannot read overlay: {error}"))? {
            let entry = entry.map_err(|error| format!("Cannot inspect overlay entry: {error}"))?;
            copy_overlay_tree(&entry.path(), &destination.join(entry.file_name()))?;
        }
    } else if metadata.is_file() {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("Cannot create overlay parent: {error}"))?;
        }
        fs::copy(source, destination).map_err(|error| format!("Cannot copy overlay file: {error}"))?;
    } else {
        return Err("Filesystem overlay contains an unsupported file type.".into());
    }
    Ok(())
}

#[tauri::command]
pub fn prepare_debian_live_project(
    workspace: String,
    project_toml: String,
) -> Result<BuildPreparation, String> {
    let root = super::workspace::canonical_root(&workspace)?;
    let project: OsProject = toml::from_str(&project_toml)
        .map_err(|error| format!("Invalid void-os.toml: {error}"))?;
    let report = validate_project(&project);
    if !report.valid {
        return Err(format!("Project validation failed: {}", report.errors.join("; ")));
    }
    if project.base.distribution != "debian" {
        return Err("This preparation backend currently supports Debian only.".into());
    }
    if project.filesystem.overlay.is_empty() {
        return Err("Set filesystem.overlay in void-os.toml before preparing the Debian image.".into());
    }

    let overlay_relative = Path::new(&project.filesystem.overlay);
    if !safe_project_relative_path(&project.filesystem.overlay) {
        return Err("Filesystem overlay must stay inside the OS project.".into());
    }

    let build_root = root.join(".void-memingo").join("debian-live").join(&project.project.id);
    fs::create_dir_all(&build_root)
        .map_err(|error| format!("Cannot create Debian build directory: {error}"))?;

    let package_list = build_root.join("config").join("package-lists").join("void-memingo.list.chroot");
    if let Some(parent) = package_list.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("Cannot create package list directory: {error}"))?;
    }
    let mut packages = project.packages.install.clone();
    if project.desktop.environment != "none" {
        let desktop_packages: &[&str] = match project.desktop.environment.as_str() {
            "gnome" => &["task-gnome-desktop"],
            "kde-plasma" => &["task-kde-desktop"],
            "xfce" => &["task-xfce-desktop"],
            "lxqt" => &["task-lxqt-desktop"],
            "cinnamon" => &["task-cinnamon-desktop"],
            "mate" => &["task-mate-desktop"],
            _ => &[],
        };
        packages.extend(desktop_packages.iter().map(|item| item.to_string()));
    }
    packages.sort();
    packages.dedup();
    fs::write(&package_list, format!("{}\n", packages.join("\n")))
        .map_err(|error| format!("Cannot write Debian package list: {error}"))?;

    let overlay_source = root.join(overlay_relative);
    let overlay_target = build_root.join("config").join("includes.chroot");
    match fs::symlink_metadata(&overlay_source) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err("Filesystem overlay cannot be a symbolic link.".into());
            }
            let canonical_overlay = fs::canonicalize(&overlay_source)
                .map_err(|error| format!("Cannot resolve filesystem overlay: {error}"))?;
            if !canonical_overlay.starts_with(&root) {
                return Err("Filesystem overlay resolves outside the OS project.".into());
            }
            copy_overlay_tree(&canonical_overlay, &overlay_target)?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(&overlay_target)
                .map_err(|error| format!("Cannot create empty root filesystem overlay: {error}"))?;
        }
        Err(error) => return Err(format!("Cannot inspect filesystem overlay: {error}")),
    }

    let iso_label = if project.project.iso_label.is_empty() {
        "VOIDMEMINGO".to_string()
    } else {
        project.project.iso_label.clone()
    };
    let hostname = if project.filesystem.hostname.is_empty() {
        "void-memingo".to_string()
    } else {
        project.filesystem.hostname.clone()
    };
    let distro = shell_quote(&project.base.release);
    let arch = shell_quote(&project.base.architecture);
    let label = shell_quote(&iso_label);
    let script = format!(
        "#!/bin/sh\nset -eu\n\n# Generated by Void Memingo. Review before running.\nlb config --distribution {distro} --architectures {arch} --binary-images iso-hybrid --archive-areas 'main contrib non-free non-free-firmware' --bootappend-live \"boot=live components hostname={hostname}\" --iso-volume {label}\nlb build\n"
    );
    let script_path = build_root.join("build.sh");
    fs::write(&script_path, script)
        .map_err(|error| format!("Cannot write Debian build script: {error}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&script_path)
            .map_err(|error| format!("Cannot inspect generated build script: {error}"))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script_path, permissions)
            .map_err(|error| format!("Cannot set build script permissions: {error}"))?;
    }

    let build_directory = build_root.to_string_lossy().into_owned();
    let script_display = script_path.to_string_lossy().into_owned();
    Ok(BuildPreparation {
        backend: "Debian Live / live-build".into(),
        build_directory: build_directory.clone(),
        script_path: script_display.clone(),
        commands: vec![
            format!("cd {}", shell_quote(&build_directory)),
            format!("sudo {}", shell_quote(&script_display)),
        ],
        notes: vec![
            "This step prepares the live-build configuration; it does not create an ISO yet.".into(),
            "Review build.sh before running it. live-build may require administrator privileges.".into(),
            "The generated script only runs lb config and lb build; no project-provided scripts are executed.".into(),
        ],
    })
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
    fn parses_the_documented_toml_manifest() {
        let manifest = r#"
[project]
name = "Atdev OS"
id = "atdev-os"
version = "0.1.0"
publisher = "Atdev Dev"
description = "Example"
iso_label = "ATDEV_OS"

[base]
distribution = "debian"
release = "stable"
architecture = "amd64"

[packages]
install = ["firefox", "git"]
remove = []

[filesystem]
overlay = "filesystem"
locale = "es_CL.UTF-8"
timezone = "America/Santiago"
hostname = "atdev-os"
storage_filesystem = "ext4"
services_enabled = []

[desktop]
environment = "xfce"
display_manager = "lightdm"

[boot]
uefi = true
bios = true
live_session = true
installer = false

[output]
iso_name = "atdev-os.iso"
directory = "dist"
"#;
        let parsed: OsProject = toml::from_str(manifest).expect("manifest should parse");
        let report = validate_project(&parsed);
        assert!(report.valid, "{:?}", report.errors);
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
