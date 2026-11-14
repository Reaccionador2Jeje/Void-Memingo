use serde::{Deserialize, Serialize};
use std::{fs, path::{Component, Path, PathBuf}, process::Command};

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
            backend_status: "preview",
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
            backend_status: "preview",
            releases: &["24.04 LTS", "26.04 LTS"],
            architectures: &["amd64"],
            desktops: &["gnome", "kde-plasma", "xfce", "lxqt", "cinnamon", "mate", "none"],
            boot_modes: &["uefi", "bios"],
            notes: "Experimental Ubuntu live-build backend with casper integration; requires a Linux host with Ubuntu-compatible live-build tooling and end-to-end validation.",
        },
        BaseCapabilities {
            id: "arch",
            name: "Arch Linux",
            backend: "archiso",
            backend_status: "preview",
            releases: &["rolling"],
            architectures: &["x86_64"],
            desktops: &["gnome", "kde-plasma", "xfce", "lxqt", "cinnamon", "mate", "none"],
            boot_modes: &["uefi", "bios"],
            notes: "Experimental native archiso releng profile backend with package lists and airootfs overlay; requires Arch Linux build tools and boot validation.",
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

#[derive(Debug, Clone, Deserialize, Serialize)]
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

// Keep Rust-created defaults consistent with the TOML deserialization default.
impl Default for DesktopConfig {
    fn default() -> Self {
        Self {
            environment: default_desktop(),
            display_manager: String::new(),
            applications: Vec::new(),
            theme: String::new(),
            icon_theme: String::new(),
            wallpaper: String::new(),
        }
    }
}

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

fn safe_iso_filename(value: &str) -> bool {
    value.len() > 4
        && value.len() <= 128
        && value.ends_with(".iso")
        && value.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
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
    if project.project.version.len() > 64
        || !project.project.version.bytes().all(|b| b.is_ascii_alphanumeric() || b"._+-".contains(&b))
    {
        errors.push("Project version contains unsupported characters.".into());
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
            match capability.backend_status {
                "ready" => {}
                "preview" => warnings.push(format!(
                    "The {} backend is in preview; generated profiles and scripts still require end-to-end ISO build and boot validation.",
                    capability.name
                )),
                _ => warnings.push(format!(
                    "The {} backend is not available yet; this project can be validated but its ISO build is not supported.",
                    capability.name
                )),
            }
        }
    }

    if !project.packages.install.iter().all(|item| safe_package_name(item))
        || !project.packages.remove.iter().all(|item| safe_package_name(item))
    {
        errors.push("Package names contain unsupported characters.".into());
    }
    if !project.desktop.applications.iter().all(|item| safe_package_name(item))
        || (!project.desktop.display_manager.is_empty() && !safe_package_name(&project.desktop.display_manager))
    {
        errors.push("Desktop package names contain unsupported characters.".into());
    }
    if project.packages.install.iter().any(|item| project.packages.remove.contains(item)) {
        errors.push("A package cannot be both installed and removed.".into());
    }
    let protected_packages: &[&str] = match base.as_str() {
        "debian" => &["live-boot", "live-config"],
        "ubuntu" => &["casper", "systemd", "systemd-sysv"],
        "arch" => &["filesystem", "glibc", "linux", "mkinitcpio", "mkinitcpio-archiso", "pacman", "systemd"],
        _ => &[],
    };
    for package in &project.packages.remove {
        if protected_packages.contains(&package.as_str()) {
            errors.push(format!("Package '{package}' is required by the selected live image backend and cannot be removed."));
        }
    }
    if !project.filesystem.services_enabled.iter().all(|service| {
        !service.is_empty() && service.len() <= 128
            && service.bytes().all(|b| b.is_ascii_alphanumeric() || b"_.@-".contains(&b))
    }) {
        errors.push("Service names contain unsupported characters.".into());
    }
    if !project.filesystem.locale.is_empty()
        && !project.filesystem.locale.bytes().all(|b| b.is_ascii_alphanumeric() || b"_.@-".contains(&b))
    {
        errors.push("Locale contains unsupported characters.".into());
    }
    if !project.filesystem.timezone.is_empty()
        && (project.filesystem.timezone.starts_with('/')
            || project.filesystem.timezone.split('/').any(|part| part == "..")
            || !project.filesystem.timezone.bytes().all(|b| b.is_ascii_alphanumeric() || b"/_+-".contains(&b)))
    {
        errors.push("Timezone must be a safe zoneinfo name such as America/Santiago.".into());
    }
    for (label, value) in [("Desktop theme", &project.desktop.theme), ("Icon theme", &project.desktop.icon_theme)] {
        if value.len() > 128 || !value.bytes().all(|b| b.is_ascii_alphanumeric() || b" _.-".contains(&b)) {
            errors.push(format!("{label} contains unsupported characters."));
        }
    }
    if !project.desktop.wallpaper.is_empty() && !safe_project_relative_path(&project.desktop.wallpaper) {
        errors.push("Wallpaper must be a safe path relative to the OS project.".into());
    }
    if !project.filesystem.storage_filesystem.is_empty()
        && !["ext4", "btrfs", "xfs"].contains(&project.filesystem.storage_filesystem.as_str())
    {
        errors.push("Storage filesystem must be ext4, btrfs or xfs.".into());
    }

    if !safe_project_relative_path(&project.filesystem.overlay) {
        errors.push("Filesystem overlay must be a safe path relative to the OS project.".into());
    }
    if !safe_project_relative_path(&project.output.directory) {
        errors.push("Output directory must be a safe path relative to the OS project.".into());
    }
    if !project.output.iso_name.is_empty() && !safe_iso_filename(&project.output.iso_name) {
        errors.push("ISO filename must be a simple .iso filename without directory components.".into());
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
pub fn check_os_build_preflight(workspace: String, project_toml: String) -> Result<BuildPreflight, String> {
    let root = super::workspace::canonical_root(&workspace)?;
    let project: OsProject = toml::from_str(&project_toml)
        .map_err(|error| format!("Invalid void-os.toml: {error}"))?;
    let distribution = project.base.distribution.clone();

    #[cfg(not(target_os = "linux"))]
    {
        return Ok(BuildPreflight {
            supported_host: false,
            ready: false,
            distribution,
            checks: vec![PreflightCheck {
                id: "linux-host".into(),
                label: "Host Linux".into(),
                available: false,
                required: true,
                detail: "La creación de imágenes está disponible únicamente desde un host Linux.".into(),
            }],
            warnings: vec!["Abre Void Memingo en Linux para ejecutar el constructor de ISO.".into()],
        });
    }

    #[cfg(target_os = "linux")]
    {
        let mut checks = Vec::new();
        for (command, package) in required_build_commands(&distribution)? {
            let available = command_available(command);
            checks.push(PreflightCheck {
                id: command.into(),
                label: package.into(),
                available,
                required: true,
                detail: if available {
                    format!("{command} está disponible en PATH.")
                } else {
                    format!("No se encontró {command}. Instala el paquete {package}.")
                },
            });
        }

        if distribution == "arch" {
            let os_release = fs::read_to_string("/etc/os-release").unwrap_or_default();
            let mut host_id = String::new();
            let mut host_like = String::new();
            for line in os_release.lines() {
                if let Some(value) = line.strip_prefix("ID=") {
                    host_id = value.trim_matches('"').to_string();
                } else if let Some(value) = line.strip_prefix("ID_LIKE=") {
                    host_like = value.trim_matches('"').to_string();
                }
            }
            let supported_host = host_id == "arch"
                || host_like.split_whitespace().any(|value| value == "arch");
            checks.push(PreflightCheck {
                id: "arch-host".into(),
                label: "Host compatible con Arch Linux".into(),
                available: supported_host,
                required: true,
                detail: if supported_host {
                    format!("Host detectado: {}.", if host_id.is_empty() { host_like } else { host_id })
                } else {
                    "Archiso requiere un host Arch Linux o una distribución que declare ID_LIKE=arch.".into()
                },
            });
            let user_namespaces = Command::new("unshare")
                .args(["-Ur", "true"])
                .status()
                .map(|status| status.success())
                .unwrap_or(false);
            checks.push(PreflightCheck {
                id: "arch-user-namespaces".into(),
                label: "Espacios de nombres de usuario".into(),
                available: user_namespaces,
                required: true,
                detail: if user_namespaces {
                    "El kernel permite ejecutar mkarchiso sin privilegios elevados.".into()
                } else {
                    "mkarchiso necesita espacios de nombres de usuario habilitados para esta ejecución sin sudo.".into()
                },
            });
        }

        let mut warnings = Vec::new();
        let disk = Command::new("df").args(["-Pk"]).arg(&root).output();
        match disk {
            Ok(output) if output.status.success() => {
                let text = String::from_utf8_lossy(&output.stdout);
                let available_kib = text.lines().last()
                    .and_then(|line| line.split_whitespace().nth(3))
                    .and_then(|value| value.parse::<u64>().ok());
                if let Some(kib) = available_kib {
                    let available_gib = kib as f64 / 1024.0 / 1024.0;
                    let enough = kib >= 15 * 1024 * 1024;
                    checks.push(PreflightCheck {
                        id: "disk-space".into(),
                        label: "Espacio libre".into(),
                        available: enough,
                        required: false,
                        detail: format!("{available_gib:.1} GiB disponibles; se recomiendan al menos 15 GiB."),
                    });
                    if !enough {
                        warnings.push("El espacio libre es inferior a 15 GiB; una compilación puede quedarse sin almacenamiento.".into());
                    }
                } else {
                    warnings.push("No se pudo interpretar el espacio libre del volumen del proyecto.".into());
                }
            }
            _ => warnings.push("No se pudo consultar el espacio libre con df.".into()),
        }

        if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
            let available_kib = meminfo.lines()
                .find(|line| line.starts_with("MemAvailable:"))
                .and_then(|line| line.split_whitespace().nth(1))
                .and_then(|value| value.parse::<u64>().ok());
            if let Some(kib) = available_kib {
                let available_gib = kib as f64 / 1024.0 / 1024.0;
                let enough = kib >= 4 * 1024 * 1024;
                checks.push(PreflightCheck {
                    id: "memory".into(),
                    label: "Memoria disponible".into(),
                    available: enough,
                    required: false,
                    detail: format!("{available_gib:.1} GiB disponibles; se recomiendan al menos 4 GiB."),
                });
                if !enough {
                    warnings.push("La memoria disponible es inferior a 4 GiB; el proceso puede ser lento o fallar.".into());
                }
            }
        }

        let ready = checks.iter().filter(|check| check.required).all(|check| check.available);
        if !ready {
            warnings.push("Faltan dependencias obligatorias. Instálalas y vuelve a ejecutar la comprobación.".into());
        }
        Ok(BuildPreflight {
            supported_host: true,
            ready,
            distribution,
            checks,
            warnings,
        })
    }
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


#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreflightCheck {
    pub id: String,
    pub label: String,
    pub available: bool,
    pub required: bool,
    pub detail: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildPreflight {
    pub supported_host: bool,
    pub ready: bool,
    pub distribution: String,
    pub checks: Vec<PreflightCheck>,
    pub warnings: Vec<String>,
}

fn command_available(command: &str) -> bool {
    #[cfg(target_os = "linux")]
    {
        if command == "livecd-rootfs" {
            return [
                "/usr/share/livecd-rootfs/live-build/auto/clean",
                "/usr/share/livecd-rootfs/live-build/auto/config",
                "/usr/share/livecd-rootfs/live-build/auto/build",
            ].iter().all(|path| Path::new(path).is_file());
        }
        Command::new("sh")
            .args(["-c", &format!("command -v {command} >/dev/null 2>&1")])
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = command;
        false
    }
}

fn required_build_commands(distribution: &str) -> Result<Vec<(&'static str, &'static str)>, String> {
    match distribution {
        "debian" => Ok(vec![
            ("lb", "live-build (lb)"),
            ("debootstrap", "debootstrap"),
            ("mksquashfs", "squashfs-tools"),
            ("xorriso", "xorriso"),
            ("rsync", "rsync"),
            ("file", "file"),
            ("sha256sum", "coreutils"),
            ("sudo", "sudo"),
        ]),
        "ubuntu" => Ok(vec![
            ("livecd-rootfs", "livecd-rootfs (Ubuntu image build scripts)"),
            ("lb", "Ubuntu live-build"),
            ("debootstrap", "debootstrap"),
            ("mksquashfs", "squashfs-tools"),
            ("xorriso", "xorriso"),
            ("rsync", "rsync"),
            ("file", "file"),
            ("sha256sum", "coreutils"),
            ("sudo", "sudo"),
        ]),
        "arch" => Ok(vec![
            ("mkarchiso", "archiso"),
            ("pacman", "pacman (Arch Linux host)"),
            ("pacstrap", "pacstrap (arch-install-scripts)"),
            ("arch-chroot", "arch-install-scripts"),
            ("bsdtar", "libarchive"),
            ("mkfs.fat", "dosfstools"),
            ("mkfs.ext4", "e2fsprogs"),
            ("mtools", "mtools"),
            ("mount", "util-linux"),
            ("unshare", "util-linux (user namespaces)"),
            ("awk", "awk"),
            ("find", "findutils"),
            ("gzip", "gzip"),
            ("openssl", "openssl"),
            ("sed", "sed"),
            ("mksquashfs", "squashfs-tools"),
            ("xorriso", "libisoburn / xorriso"),
            ("zstd", "zstd"),
            ("file", "file"),
            ("sha256sum", "coreutils"),
        ]),
        other => Err(format!("Unsupported Linux image backend: {other}")),
    }
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

fn resolve_project_path_no_symlinks(root: &Path, relative: &str) -> Result<Option<PathBuf>, String> {
    if !safe_project_relative_path(relative) || relative.is_empty() {
        return Err("Overlay path must be a safe relative path inside the OS project.".into());
    }
    let mut current = root.to_path_buf();
    for component in Path::new(relative).components() {
        let Component::Normal(part) = component else {
            return Err("Overlay path contains an unsafe component.".into());
        };
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    return Err("Filesystem overlay paths cannot traverse symbolic links.".into());
                }
                let canonical = fs::canonicalize(&current)
                    .map_err(|error| format!("Cannot resolve overlay path: {error}"))?;
                if !canonical.starts_with(root) {
                    return Err("Filesystem overlay resolves outside the OS project.".into());
                }
                current = canonical;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(format!("Cannot inspect overlay path: {error}")),
        }
    }
    Ok(Some(current))
}

fn safe_create_project_directory(root: &Path, relative: &str) -> Result<PathBuf, String> {
    if !safe_project_relative_path(relative) || relative.is_empty() {
        return Err("Generated directory must be a safe path inside the OS project.".into());
    }
    let mut current = root.to_path_buf();
    for component in Path::new(relative).components() {
        let Component::Normal(part) = component else {
            return Err("Generated directory contains an unsafe path component.".into());
        };
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    return Err("Generated build path contains a symlink or non-directory.".into());
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&current)
                    .map_err(|error| format!("Cannot create build directory: {error}"))?;
            }
            Err(error) => return Err(format!("Cannot inspect build directory: {error}")),
        }
        let canonical = fs::canonicalize(&current)
            .map_err(|error| format!("Cannot resolve build directory: {error}"))?;
        if !canonical.starts_with(root) {
            return Err("Generated build directory escaped the OS project.".into());
        }
        current = canonical;
    }
    Ok(current)
}

fn reject_existing_output(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(format!("Output already exists and will not be overwritten: {}", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Cannot inspect output path: {error}")),
    }
}

fn reject_symlink_target(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err("Generated build files cannot overwrite symbolic links.".into())
        }
        Ok(metadata) if !metadata.is_file() => {
            Err("Generated build file target is not a regular file.".into())
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Cannot inspect generated file target: {error}")),
    }
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
    if let Ok(destination_metadata) = fs::symlink_metadata(destination) {
        if destination_metadata.file_type().is_symlink() {
            return Err("Generated overlay destination cannot be a symbolic link.".into());
        }
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



fn apply_common_filesystem_defaults(overlay: &Path, project: &OsProject) -> Result<(), String> {
    if !project.filesystem.hostname.is_empty() {
        let etc = safe_create_project_directory(overlay, "etc")?;
        fs::write(etc.join("hostname"), format!("{}\n", project.filesystem.hostname))
            .map_err(|error| format!("Cannot write hostname: {error}"))?;
    }
    if !project.filesystem.timezone.is_empty() {
        let etc = safe_create_project_directory(overlay, "etc")?;
        fs::write(etc.join("timezone"), format!("{}\n", project.filesystem.timezone))
            .map_err(|error| format!("Cannot write timezone: {error}"))?;
    }
    if !project.filesystem.locale.is_empty() {
        let defaults = safe_create_project_directory(overlay, "etc/default")?;
        fs::write(defaults.join("locale"), format!("LANG={}\n", project.filesystem.locale))
            .map_err(|error| format!("Cannot write locale defaults: {error}"))?;
        let etc = safe_create_project_directory(overlay, "etc")?;
        fs::write(etc.join("locale.conf"), format!("LANG={}\n", project.filesystem.locale))
            .map_err(|error| format!("Cannot write locale.conf: {error}"))?;
        fs::write(etc.join("locale.gen"), format!("{} UTF-8\n", project.filesystem.locale))
            .map_err(|error| format!("Cannot write locale generation config: {error}"))?;
    }
    Ok(())
}
fn apply_desktop_defaults(root: &Path, overlay: &Path, project: &OsProject) -> Result<(), String> {
    if !project.desktop.theme.is_empty() || !project.desktop.icon_theme.is_empty() {
        let gtk_dir = safe_create_project_directory(overlay, "etc/skel/.config/gtk-3.0")?;
        let mut settings = String::from("[Settings]\n");
        if !project.desktop.theme.is_empty() {
            settings.push_str(&format!("gtk-theme-name={}\n", project.desktop.theme));
        }
        if !project.desktop.icon_theme.is_empty() {
            settings.push_str(&format!("gtk-icon-theme-name={}\n", project.desktop.icon_theme));
        }
        fs::write(gtk_dir.join("settings.ini"), settings)
            .map_err(|error| format!("Cannot write GTK desktop defaults: {error}"))?;
    }
    if !project.desktop.wallpaper.is_empty() {
        let source = resolve_project_path_no_symlinks(root, &project.desktop.wallpaper)?
            .ok_or_else(|| format!("Wallpaper '{}' does not exist.", project.desktop.wallpaper))?;
        if !source.is_file() {
            return Err("Wallpaper must point to a regular file.".into());
        }
        let extension = source.extension().and_then(|value| value.to_str()).unwrap_or("img");
        let name = format!("wallpaper.{extension}");
        let destination = safe_create_project_directory(overlay, "usr/share/backgrounds/void-memingo")?.join(&name);
        fs::copy(source, destination).map_err(|error| format!("Cannot copy wallpaper into image: {error}"))?;
        let profile = safe_create_project_directory(overlay, "etc/skel/.config/void-memingo")?;
        fs::write(profile.join("desktop-profile.conf"), format!(
            "theme={}\nicon_theme={}\nwallpaper=/usr/share/backgrounds/void-memingo/{name}\n",
            project.desktop.theme, project.desktop.icon_theme
        )).map_err(|error| format!("Cannot write desktop profile: {error}"))?;
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
    if !project.boot.uefi || !project.boot.bios || !project.boot.live_session || project.boot.installer {
        return Err("The initial Debian Live profile requires UEFI + BIOS hybrid boot, a live session, and no installer. Installer support is a later milestone.".into());
    }
    if project.filesystem.overlay.is_empty() {
        return Err("Set filesystem.overlay in void-os.toml before preparing the Debian image.".into());
    }

    if !safe_project_relative_path(&project.filesystem.overlay) {
        return Err("Filesystem overlay must stay inside the OS project.".into());
    }

    let build_relative = format!(".void-memingo/debian-live/{}", project.project.id);
    let build_root = safe_create_project_directory(&root, &build_relative)?;
    let output_relative = if project.output.directory.is_empty() { "dist" } else { &project.output.directory };
    let output_root = safe_create_project_directory(&root, output_relative)?;
    let output_name = if project.output.iso_name.is_empty() {
        format!("{}-{}.iso", project.project.id, project.project.version)
    } else {
        project.output.iso_name.clone()
    };
    if !safe_iso_filename(&output_name) {
        return Err("Generated ISO filename is invalid.".into());
    }
    let output_file = output_root.join(&output_name);
    reject_existing_output(&output_file)?;
    let output_checksum = output_root.join(format!("{output_name}.sha256"));
    reject_existing_output(&output_checksum)?;
    let config_root = safe_create_project_directory(&root, &format!("{build_relative}/config"))?;
    let package_list_root = safe_create_project_directory(&root, &format!("{build_relative}/config/package-lists"))?;
    let package_list = package_list_root.join("void-memingo.list.chroot");
    reject_symlink_target(&package_list)?;
    let mut packages = project.packages.install.clone();
    packages.extend(project.desktop.applications.iter().cloned());
    if !project.desktop.display_manager.is_empty() {
        packages.push(project.desktop.display_manager.clone());
    }
    if !project.filesystem.locale.is_empty() {
        packages.push("locales".into());
    }
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

    let overlay_target = config_root.join("includes.chroot");
    if let Ok(metadata) = fs::symlink_metadata(&overlay_target) {
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err("Generated root filesystem overlay target is not a safe directory.".into());
        }
        fs::remove_dir_all(&overlay_target)
            .map_err(|error| format!("Cannot refresh generated root filesystem overlay: {error}"))?;
    }
    match resolve_project_path_no_symlinks(&root, &project.filesystem.overlay)? {
        Some(canonical_overlay) => copy_overlay_tree(&canonical_overlay, &overlay_target)?,
        None => {
            fs::create_dir_all(&overlay_target)
                .map_err(|error| format!("Cannot create empty root filesystem overlay: {error}"))?;
        }
    }
    apply_desktop_defaults(&root, &overlay_target, &project)?;

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
    let output_dir = shell_quote(&output_root.to_string_lossy());
    let output_file_arg = shell_quote(&output_file.to_string_lossy());
    let checksum_file_arg = shell_quote(&output_checksum.to_string_lossy());
    let script = format!(
        "#!/bin/sh\nset -eu\n\n# Generated by Void Memingo. Review before running.\nlb config --distribution {distro} --architectures {arch} --binary-images iso-hybrid --archive-areas 'main contrib non-free non-free-firmware' --bootappend-live \"boot=live components hostname={hostname}\" --iso-volume {label}\nlb build\niso_count=0\niso_path=''\nfor candidate in ./*.iso; do\n  if [ -f \"$candidate\" ]; then iso_count=$((iso_count + 1)); iso_path=$candidate; fi\ndone\nif [ \"$iso_count\" -ne 1 ]; then echo 'Expected exactly one ISO from live-build.' >&2; exit 1; fi\nmkdir -p {output_dir}\nif [ -e {output_file_arg} ] || [ -e {checksum_file_arg} ]; then echo 'Output already exists; refusing to overwrite.' >&2; exit 1; fi\ncp \"$iso_path\" {output_file_arg}\nfile {output_file_arg} | grep -qi 'ISO 9660' || {{ echo 'Output is not an ISO 9660 image.' >&2; exit 1; }}\nsha256sum {output_file_arg} > {checksum_file_arg}\nsha256sum --check {checksum_file_arg}\necho 'ISO validated and SHA-256 checksum created.'\n"
    );
    let hook_root = safe_create_project_directory(&root, &format!("{build_relative}/config/hooks/normal"))?;
    if !project.packages.remove.is_empty() {
        let remove_hook = hook_root.join("015-void-memingo-remove-packages.hook.chroot");
        reject_symlink_target(&remove_hook)?;
        let packages = project.packages.remove.iter().map(|name| shell_quote(name)).collect::<Vec<_>>().join(" ");
        let script = format!("#!/bin/sh\nset -eu\napt-get purge --yes -- {packages}\napt-get autoremove --yes\n");
        fs::write(&remove_hook, script)
            .map_err(|error| format!("Cannot write Debian package removal hook: {error}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&remove_hook)
                .map_err(|error| format!("Cannot inspect package removal hook: {error}"))?
                .permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&remove_hook, permissions)
                .map_err(|error| format!("Cannot set package removal hook permissions: {error}"))?;
        }
    }

    let hook_path = hook_root.join("020-void-memingo-system-config.hook.chroot");
    reject_symlink_target(&hook_path)?;
    let mut hook = String::from("#!/bin/sh\nset -eu\n");
    if !project.filesystem.hostname.is_empty() {
        hook.push_str(&format!("printf '%s\\n' {} > /etc/hostname\n", shell_quote(&project.filesystem.hostname)));
    }
    if !project.filesystem.timezone.is_empty() {
        hook.push_str(&format!("printf '%s\\n' {} > /etc/timezone\n", shell_quote(&project.filesystem.timezone)));
        hook.push_str(&format!("if [ -e /usr/share/zoneinfo/{0} ]; then ln -snf {0} /etc/localtime; fi\n", project.filesystem.timezone));
    }
    if !project.filesystem.locale.is_empty() {
        hook.push_str(&format!("printf '%s\\n' {} > /etc/default/locale\n", shell_quote(&format!("LANG={}", project.filesystem.locale))));
        hook.push_str(&format!("grep -qxF {} /etc/locale.gen || printf '%s\\n' {} >> /etc/locale.gen\n", shell_quote(&format!("{} UTF-8", project.filesystem.locale)), shell_quote(&format!("{} UTF-8", project.filesystem.locale))));
        hook.push_str("locale-gen\n");
    }
    for service in &project.filesystem.services_enabled {
        hook.push_str(&format!("systemctl enable {}\n", shell_quote(service)));
    }
    fs::write(&hook_path, hook)
        .map_err(|error| format!("Cannot write Debian system configuration hook: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&hook_path)
            .map_err(|error| format!("Cannot inspect generated system hook: {error}"))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&hook_path, permissions)
            .map_err(|error| format!("Cannot set system hook permissions: {error}"))?;
    }

    let script_path = build_root.join("build.sh");
    reject_symlink_target(&script_path)?;
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
            "The generated script only invokes lb config and lb build. Review any custom live-build hooks in config/hooks before running; those hooks execute inside the target chroot.".into(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

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
    fn reports_debian_backend_as_preview_not_unimplemented() {
        let report = validate_project(&sample());
        assert!(report.warnings.iter().any(|warning| warning.contains("in preview")));
        assert!(!report.warnings.iter().any(|warning| warning.contains("not available yet")));
    }

    #[test]
    fn reports_ubuntu_backend_as_experimental_preview() {
        let mut project = sample();
        project.base.distribution = "ubuntu".into();
        project.base.release = "24.04 LTS".into();
        let report = validate_project(&project);
        assert!(report.warnings.iter().any(|warning| warning.contains("in preview")));
        assert!(!report.warnings.iter().any(|warning| warning.contains("not available yet")));
    }

    #[test]
    fn reports_arch_backend_as_experimental_preview() {
        let mut project = sample();
        project.base.distribution = "arch".into();
        project.base.release = "rolling".into();
        project.base.architecture = "x86_64".into();
        let report = validate_project(&project);
        assert!(report.warnings.iter().any(|warning| warning.contains("in preview")));
        assert!(!report.warnings.iter().any(|warning| warning.contains("not available yet")));
    }

    #[test]
    fn rejects_removing_backend_critical_packages() {
        let mut project = sample();
        project.packages.remove.push("live-boot".into());
        assert!(!validate_project(&project).valid);

        project.base.distribution = "arch".into();
        project.base.release = "rolling".into();
        project.base.architecture = "x86_64".into();
        project.packages.remove.clear();
        project.packages.remove.push("mkinitcpio-archiso".into());
        assert!(!validate_project(&project).valid);
    }

    #[test]
    fn backend_dependencies_are_distribution_specific() {
        let ubuntu = required_build_commands("ubuntu").expect("Ubuntu backend");
        let arch = required_build_commands("arch").expect("Arch backend");
        assert!(ubuntu.iter().any(|(command, _)| *command == "livecd-rootfs"));
        assert!(ubuntu.iter().any(|(command, _)| *command == "lb"));
        assert!(!ubuntu.iter().any(|(command, _)| *command == "mkarchiso"));
        assert!(arch.iter().any(|(command, _)| *command == "mkarchiso"));
        assert!(arch.iter().any(|(command, _)| *command == "pacstrap"));
        assert!(arch.iter().any(|(command, _)| *command == "unshare"));
        assert!(!arch.iter().any(|(command, _)| *command == "lb"));
        assert!(!arch.iter().any(|(command, _)| *command == "sudo"));
    }

    fn temporary_os_project(distribution: &str, release: &str, architecture: &str) -> (PathBuf, OsProject) {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("void-memingo-{distribution}-{}-{nonce}", std::process::id()));
        fs::create_dir_all(root.join("filesystem")).unwrap();
        let mut project = sample();
        project.base.distribution = distribution.into();
        project.base.release = release.into();
        project.base.architecture = architecture.into();
        project.filesystem.overlay = "filesystem".into();
        project.filesystem.locale = "es_CL.UTF-8".into();
        project.filesystem.timezone = "America/Santiago".into();
        (root, project)
    }

    #[test]
    fn generates_ubuntu_livecd_rootfs_backend() {
        let (root, project) = temporary_os_project("ubuntu", "24.04 LTS", "amd64");
        let root = fs::canonicalize(root).unwrap();
        let prepared = prepare_non_debian_project(&root, &project).expect("Ubuntu backend profile");
        let script = fs::read_to_string(&prepared.script_path).unwrap();
        assert!(script.contains("/usr/share/livecd-rootfs/live-build/auto/config"));
        assert!(script.contains("PROJECT=ubuntu SUBPROJECT=live SUITE=noble"));
        assert!(script.contains("livecd.ubuntu.iso"));
        assert!(script.contains("config/includes.chroot"));
        assert!(script.contains("void-memingo.list.chroot"));
        assert!(script.contains("locale-gen"));
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn generates_archiso_releng_backend_and_customizations() {
        let (root, project) = temporary_os_project("arch", "rolling", "x86_64");
        let root = fs::canonicalize(root).unwrap();
        let prepared = prepare_non_debian_project(&root, &project).expect("Archiso backend profile");
        let script = fs::read_to_string(&prepared.script_path).unwrap();
        assert!(script.contains("/usr/share/archiso/configs/releng"));
        assert!(script.contains("mkarchiso -v"));
        assert!(script.contains("iso_name=%s"));
        assert!(script.contains("packages.x86_64"));
        assert!(script.contains("unshare"));
        assert!(prepared.commands[1].starts_with("sh "));
        assert!(root.join(".void-memingo/archiso/atdev-os/profile/custom-airootfs/etc/pacman.d/hooks/00-void-memingo-locale-gen.hook").is_file());
        assert!(root.join(".void-memingo/archiso/atdev-os/profile/custom-airootfs/etc/locale.conf").is_file());
        fs::remove_dir_all(root).unwrap();
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

    #[test]
    fn accepts_only_simple_iso_output_names() {
        assert!(safe_iso_filename("atdev-os-0.1.0.iso"));
        assert!(!safe_iso_filename("../outside.iso"));
        assert!(!safe_iso_filename("bad name.iso"));
        assert!(!safe_iso_filename(".iso"));
    }
}


fn desktop_packages_for(distribution: &str, environment: &str) -> Vec<&'static str> {
    match (distribution, environment) {
        (_, "none") => vec![],
        ("debian", "gnome") => vec!["task-gnome-desktop"],
        ("debian", "kde-plasma") => vec!["task-kde-desktop"],
        ("debian", "xfce") => vec!["task-xfce-desktop"],
        ("debian", "lxqt") => vec!["task-lxqt-desktop"],
        ("debian", "cinnamon") => vec!["task-cinnamon-desktop"],
        ("debian", "mate") => vec!["task-mate-desktop"],
        ("ubuntu", "gnome") => vec!["ubuntu-desktop-minimal"],
        ("ubuntu", "kde-plasma") => vec!["kubuntu-desktop"],
        ("ubuntu", "xfce") => vec!["xubuntu-desktop"],
        ("ubuntu", "lxqt") => vec!["lubuntu-desktop"],
        ("ubuntu", "cinnamon") => vec!["ubuntu-cinnamon-desktop"],
        ("ubuntu", "mate") => vec!["ubuntu-mate-desktop"],
        ("arch", "gnome") => vec!["gnome", "gnome-extra"],
        ("arch", "kde-plasma") => vec!["plasma", "konsole"],
        ("arch", "xfce") => vec!["xfce4", "xfce4-goodies"],
        ("arch", "lxqt") => vec!["lxqt"],
        ("arch", "cinnamon") => vec!["cinnamon"],
        ("arch", "mate") => vec!["mate", "mate-extra"],
        _ => vec![],
    }
}

fn prepare_non_debian_project(root: &Path, project: &OsProject) -> Result<BuildPreparation, String> {
    let distribution = project.base.distribution.as_str();
    let backend = match distribution {
        "ubuntu" => "Ubuntu Live / casper",
        "arch" => "Archiso",
        _ => return Err("Unsupported Linux image backend.".into()),
    };
    if !project.boot.uefi || !project.boot.bios || !project.boot.live_session || project.boot.installer {
        return Err("The current live profile requires UEFI + BIOS hybrid boot, live session enabled and installer disabled.".into());
    }
    if project.filesystem.overlay.is_empty() || !safe_project_relative_path(&project.filesystem.overlay) {
        return Err("Configure a safe filesystem.overlay path inside the OS project.".into());
    }

    let backend_slug = if distribution == "ubuntu" { "ubuntu-live" } else { "archiso" };
    let build_relative = format!(".void-memingo/{backend_slug}/{}", project.project.id);
    let build_root = safe_create_project_directory(root, &build_relative)?;
    let output_relative = if project.output.directory.is_empty() { "dist" } else { &project.output.directory };
    let output_root = safe_create_project_directory(root, output_relative)?;
    let output_name = if project.output.iso_name.is_empty() {
        format!("{}-{}.iso", project.project.id, project.project.version)
    } else {
        project.output.iso_name.clone()
    };
    if !safe_iso_filename(&output_name) {
        return Err("Generated ISO filename is invalid.".into());
    }
    let output_file = output_root.join(&output_name);
    reject_existing_output(&output_file)?;
    let checksum_file = output_root.join(format!("{output_name}.sha256"));
    reject_existing_output(&checksum_file)?;

    let source_overlay = resolve_project_path_no_symlinks(root, &project.filesystem.overlay)?
        .ok_or_else(|| format!("Filesystem overlay '{}' does not exist.", project.filesystem.overlay))?;
    let profile_root = if distribution == "arch" {
        safe_create_project_directory(root, &format!("{build_relative}/profile"))?
    } else {
        build_root.clone()
    };

    let mut packages = project.packages.install.clone();
    packages.extend(project.desktop.applications.iter().cloned());
    packages.extend(desktop_packages_for(distribution, &project.desktop.environment).iter().map(|p| p.to_string()));
    if project.filesystem.services_enabled.iter().any(|service| service == "NetworkManager.service") {
        packages.push(if distribution == "arch" { "networkmanager".into() } else { "network-manager".into() });
    }
    if distribution == "ubuntu" {
        packages.push("casper".into());
        packages.push("discover".into());
        packages.push("locales".into());
    } else if distribution == "arch" {
        packages.extend(["linux", "linux-firmware", "mkinitcpio", "mkinitcpio-archiso", "sudo"].iter().map(|p| p.to_string()));
        if !project.filesystem.locale.is_empty() { packages.push("glibc".into()); }
        match project.desktop.environment.as_str() {
            "gnome" => packages.push("gdm".into()),
            "kde-plasma" => packages.push("sddm".into()),
            "xfce" | "lxqt" | "cinnamon" | "mate" => {
                packages.push("lightdm".into());
                packages.push("lightdm-gtk-greeter".into());
            },
            _ => {}
        }
    }
    packages.sort();
    packages.dedup();

    let overlay_target = if distribution == "arch" {
        safe_create_project_directory(root, &format!("{build_relative}/profile/custom-airootfs"))?
    } else {
        safe_create_project_directory(root, &format!("{build_relative}/overlay"))?
    };
    if let Ok(metadata) = fs::symlink_metadata(&overlay_target) {
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err("Generated filesystem overlay is not a safe directory.".into());
        }
        fs::remove_dir_all(&overlay_target)
            .map_err(|error| format!("Cannot refresh generated filesystem overlay: {error}"))?;
    }
    fs::create_dir_all(&overlay_target)
        .map_err(|error| format!("Cannot create generated filesystem overlay: {error}"))?;
    copy_overlay_tree(&source_overlay, &overlay_target)?;
    apply_common_filesystem_defaults(&overlay_target, project)?;
    apply_desktop_defaults(root, &overlay_target, project)?;

    if distribution == "ubuntu" {
        let hook_root = safe_create_project_directory(root, &format!("{build_relative}/hooks"))?;
        if let Ok(metadata) = fs::symlink_metadata(&hook_root) {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err("Generated Ubuntu hooks path is not a safe directory.".into());
            }
            fs::remove_dir_all(&hook_root)
                .map_err(|error| format!("Cannot refresh Ubuntu hooks: {error}"))?;
        }
        fs::create_dir_all(&hook_root)
            .map_err(|error| format!("Cannot create Ubuntu hooks: {error}"))?;
        if !project.packages.remove.is_empty() {
            let remove_hook = hook_root.join("015-void-memingo-remove-packages.hook.chroot");
            reject_symlink_target(&remove_hook)?;
            let names = project.packages.remove.iter().map(|name| shell_quote(name)).collect::<Vec<_>>().join(" ");
            fs::write(&remove_hook, format!("#!/bin/sh\nset -eu\napt-get purge --yes -- {names}\napt-get autoremove --yes\n"))
                .map_err(|error| format!("Cannot write Ubuntu package removal hook: {error}"))?;
        }
        let service_hook = hook_root.join("020-void-memingo-system-config.hook.chroot");
        reject_symlink_target(&service_hook)?;
        let mut hook = String::from("#!/bin/sh\nset -eu\n");
        if !project.filesystem.hostname.is_empty() {
            hook.push_str(&format!("printf '%s\\n' {} > /etc/hostname\n", shell_quote(&project.filesystem.hostname)));
        }
        if !project.filesystem.timezone.is_empty() {
            hook.push_str(&format!("printf '%s\\n' {} > /etc/timezone\n", shell_quote(&project.filesystem.timezone)));
            hook.push_str(&format!("if [ -e /usr/share/zoneinfo/{0} ]; then ln -snf {0} /etc/localtime; fi\n", project.filesystem.timezone));
        }
        if !project.filesystem.locale.is_empty() {
            let locale_line = format!("{} UTF-8", project.filesystem.locale);
            hook.push_str(&format!("grep -qxF {} /etc/locale.gen || printf '%s\\n' {} >> /etc/locale.gen\n", shell_quote(&locale_line), shell_quote(&locale_line)));
            hook.push_str(&format!("printf '%s\\n' {} > /etc/default/locale\n", shell_quote(&format!("LANG={}", project.filesystem.locale))));
            hook.push_str("locale-gen\n");
        }
        for service in &project.filesystem.services_enabled {
            hook.push_str(&format!("systemctl enable {}\n", shell_quote(service)));
        }
        fs::write(&service_hook, hook).map_err(|error| format!("Cannot write Ubuntu system configuration hook: {error}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for path in [hook_root.join("015-void-memingo-remove-packages.hook.chroot"), service_hook] {
                if path.exists() {
                    let mut permissions = fs::metadata(&path).map_err(|error| format!("Cannot inspect Ubuntu hook: {error}"))?.permissions();
                    permissions.set_mode(0o755);
                    fs::set_permissions(&path, permissions).map_err(|error| format!("Cannot mark Ubuntu hook executable: {error}"))?;
                }
            }
        }
    } else if distribution == "arch" {
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let mut services = project.filesystem.services_enabled.clone();
            let display_manager = match project.desktop.environment.as_str() {
                "gnome" => Some("gdm.service"),
                "kde-plasma" => Some("sddm.service"),
                "xfce" | "lxqt" | "cinnamon" | "mate" => Some("lightdm.service"),
                _ => None,
            };
            if let Some(service) = display_manager {
                services.push(service.into());
            }
            services.sort();
            services.dedup();
            if !services.is_empty() {
                let wants = safe_create_project_directory(&overlay_target, "etc/systemd/system/multi-user.target.wants")?;
                for service in services {
                    let link = wants.join(&service);
                    match fs::symlink_metadata(&link) {
                        Ok(metadata) if metadata.file_type().is_symlink() => continue,
                        Ok(_) => return Err(format!("Service path already exists and is not a symlink: {service}")),
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                        Err(error) => return Err(format!("Cannot inspect Arch service path {service}: {error}")),
                    }
                    symlink(format!("/usr/lib/systemd/system/{service}"), link)
                        .map_err(|error| format!("Cannot enable Arch service {service}: {error}"))?;
                }
            }
            if !project.filesystem.timezone.is_empty() {
                let etc = safe_create_project_directory(&overlay_target, "etc")?;
                let localtime = etc.join("localtime");
                if fs::symlink_metadata(&localtime).is_ok() {
                    return Err("The Arch overlay already contains /etc/localtime; remove it or leave timezone empty.".into());
                }
                symlink(format!("/usr/share/zoneinfo/{}", project.filesystem.timezone), localtime)
                    .map_err(|error| format!("Cannot configure Arch timezone: {error}"))?;
            }
            if !project.filesystem.locale.is_empty() {
                let hook_root = safe_create_project_directory(&overlay_target, "etc/pacman.d/hooks")?;
                let locale_hook = hook_root.join("00-void-memingo-locale-gen.hook");
                let hook = "# remove from airootfs!\n[Trigger]\nOperation = Install\nOperation = Upgrade\nType = Package\nTarget = glibc\n\n[Action]\nDescription = Generate configured locales\nWhen = PostTransaction\nDepends = glibc\nExec = /usr/bin/locale-gen\n";
                fs::write(&locale_hook, hook)
                    .map_err(|error| format!("Cannot write Arch locale generation hook: {error}"))?;
            }
        }
        #[cfg(not(unix))]
        return Err("Arch profile generation requires a Unix host for systemd links and timezone configuration.".into());
    }



    let script_path = build_root.join("build.sh");
    reject_symlink_target(&script_path)?;
    let output_q = shell_quote(&output_file.to_string_lossy());
    let checksum_q = shell_quote(&checksum_file.to_string_lossy());
    let output_dir_q = shell_quote(&output_root.to_string_lossy());
    let iso_label = if project.project.iso_label.is_empty() { "VOIDMEMINGO" } else { &project.project.iso_label };
    let script = if distribution == "ubuntu" {
        let codename = match project.base.release.as_str() {
            "24.04 LTS" => "noble",
            "26.04 LTS" => "resolute",
            other => return Err(format!("Unsupported Ubuntu release: {other}")),
        };
        let package_file = build_root.join("packages.list");
        let hooks_dir = build_root.join("hooks");
        reject_symlink_target(&package_file)?;
        fs::write(&package_file, format!("{}\n", packages.join("\n")))
            .map_err(|error| format!("Cannot write Ubuntu package list: {error}"))?;
        format!(r#"#!/bin/sh
set -eu
AUTO=/usr/share/livecd-rootfs/live-build/auto
[ -x "$AUTO/clean" ] && [ -x "$AUTO/config" ] && [ -x "$AUTO/build" ] || {{ echo "Install the Ubuntu livecd-rootfs package." >&2; exit 127; }}
command -v lb >/dev/null 2>&1 || {{ echo "Install Ubuntu live-build." >&2; exit 127; }}
command -v debootstrap >/dev/null 2>&1 || {{ echo "Install debootstrap." >&2; exit 127; }}
cd {build_dir}
find . -maxdepth 1 -type f -name 'livecd.ubuntu.iso' -delete
ARCH=amd64 PROJECT=ubuntu SUBPROJECT=live SUITE={codename} "$AUTO/clean"
ARCH=amd64 PROJECT=ubuntu SUBPROJECT=live SUITE={codename} "$AUTO/config"
mkdir -p config/package-lists config/includes.chroot config/hooks/normal
cp {packages_file} config/package-lists/void-memingo.list.chroot
cp -a {overlay}/. config/includes.chroot/
if [ -d {hooks} ]; then cp -a {hooks}/. config/hooks/normal/; fi
lb config --iso-volume {label}
ARCH=amd64 PROJECT=ubuntu SUBPROJECT=live SUITE={codename} "$AUTO/build"
image=livecd.ubuntu.iso
[ -s "$image" ] || {{ echo "livecd-rootfs did not produce livecd.ubuntu.iso." >&2; exit 1; }}
mkdir -p {output_dir}
[ ! -e {output} ] && [ ! -e {checksum} ] || {{ echo "Refusing to overwrite an existing output." >&2; exit 1; }}
install -m 0644 "$image" {output}
file {output} | grep -qi "ISO 9660" || {{ echo "Output is not an ISO 9660 image." >&2; exit 1; }}
sha256sum {output} > {checksum}
sha256sum --check {checksum}
echo "Ubuntu live ISO validated and SHA-256 checksum created."
"#,
            build_dir = shell_quote(&build_root.to_string_lossy()),
            codename = codename,
            packages_file = shell_quote(&package_file.to_string_lossy()),
            overlay = shell_quote(&overlay_target.to_string_lossy()),
            hooks = shell_quote(&hooks_dir.to_string_lossy()),
            label = shell_quote(iso_label),
            output_dir = output_dir_q,
            output = output_q,
            checksum = checksum_q,
        )
    } else {
        let package_file = profile_root.join("packages.x86_64");
        reject_symlink_target(&package_file)?;
        let arch_profile = safe_create_project_directory(root, &format!("{build_relative}/profile/archiso-profile"))?;
        fs::write(&package_file, format!("{}\n", packages.join("\n")))
            .map_err(|error| format!("Cannot write Arch package list: {error}"))?;
        let profile_path = arch_profile.to_string_lossy().to_string();
        let packages_path = package_file.to_string_lossy().to_string();
        let remove_file = build_root.join("packages.remove");
        reject_symlink_target(&remove_file)?;
        fs::write(&remove_file, format!("{}\n", project.packages.remove.join("\n")))
            .map_err(|error| format!("Cannot write Arch package removal list: {error}"))?;
        format!(r#"#!/bin/sh
set -eu
command -v mkarchiso >/dev/null 2>&1 || {{ echo "Install archiso first." >&2; exit 127; }}
command -v pacman >/dev/null 2>&1 || {{ echo "Archiso must run on an Arch Linux build host with pacman." >&2; exit 127; }}
command -v pacstrap >/dev/null 2>&1 || {{ echo "Install arch-install-scripts first." >&2; exit 127; }}
command -v unshare >/dev/null 2>&1 || {{ echo "Install util-linux with unshare support." >&2; exit 127; }}
. /etc/os-release
case " ${{ID:-}} ${{ID_LIKE:-}} " in *" arch "*) ;; *) echo "Archiso requires an Arch Linux compatible build host." >&2; exit 1 ;; esac
[ -d /usr/share/archiso/configs/releng ] || {{ echo "Archiso releng profile not found." >&2; exit 1; }}
cp -a /usr/share/archiso/configs/releng/. {profile}/
printf '\niso_name=%s\niso_version=%s\niso_label=%s\niso_publisher=%s\n' {iso_name} {iso_version} {iso_label} {iso_publisher} >> {profile}/profiledef.sh
cat {packages} >> {profile}/packages.x86_64
sort -u {profile}/packages.x86_64 -o {profile}/packages.x86_64
if [ -s {remove_packages} ]; then grep -Fvx -f {remove_packages} {profile}/packages.x86_64 > {profile}/packages.tmp || true; mv {profile}/packages.tmp {profile}/packages.x86_64; fi
mkdir -p {profile}/airootfs
cp -a {custom_overlay}/. {profile}/airootfs/
mkdir -p {output_dir}
find {output_dir} -maxdepth 1 -type f -name '*.iso' -delete
mkarchiso -v -w {work}/work -o {output_dir} {profile}
count=0
image=""
for candidate in {output_dir}/*.iso; do [ -f "$candidate" ] || continue; count=$((count + 1)); image="$candidate"; done
[ "$count" -eq 1 ] && [ -s "$image" ] || {{ echo "Expected exactly one non-empty Archiso image." >&2; exit 1; }}
mkdir -p {final_dir}
[ ! -e {output} ] && [ ! -e {checksum} ] || {{ echo "Refusing to overwrite an existing output." >&2; exit 1; }}
install -m 0644 "$image" {output}
file {output} | grep -qi "ISO 9660" || {{ echo "Output is not an ISO 9660 image." >&2; exit 1; }}
sha256sum {output} > {checksum}
sha256sum --check {checksum}
echo "Arch Linux ISO validated and SHA-256 checksum created."
"#,
            profile = shell_quote(&profile_path),
            iso_name = shell_quote(&project.project.id),
            iso_version = shell_quote(&project.project.version),
            iso_label = shell_quote(iso_label),
            iso_publisher = shell_quote(if project.project.publisher.is_empty() { "Void Memingo" } else { &project.project.publisher }),
            custom_overlay = shell_quote(&overlay_target.to_string_lossy()),
            packages = shell_quote(&packages_path),
            remove_packages = shell_quote(&remove_file.to_string_lossy()),
            work = shell_quote(&build_root.to_string_lossy()),
            output_dir = shell_quote(&build_root.join("output").to_string_lossy()),
            final_dir = output_dir_q,
            output = output_q,
            checksum = checksum_q,
        )
    };
    fs::write(&script_path, script).map_err(|error| format!("Cannot write build script: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&script_path).map_err(|error| format!("Cannot inspect build script: {error}"))?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script_path, permissions).map_err(|error| format!("Cannot set build script permissions: {error}"))?;
    }
    Ok(BuildPreparation {
        backend: backend.into(),
        build_directory: build_root.to_string_lossy().into_owned(),
        script_path: script_path.to_string_lossy().into_owned(),
        commands: vec![
            format!("cd {}", shell_quote(&build_root.to_string_lossy())),
            if distribution == "arch" {
                format!("sh {}", shell_quote(&script_path.to_string_lossy()))
            } else {
                format!("sudo {}", shell_quote(&script_path.to_string_lossy()))
            },
        ],
        notes: vec![
            "This prepares a real distribution-specific backend workspace and executable build script; ISO creation starts only when the reviewed script is run.".into(),
            "Ubuntu backend: live-build package lists, casper live boot configuration, root filesystem overlay and Ubuntu release codename mapping.".into(),
            "Arch backend: upstream archiso releng profile, x86_64 package manifest, custom airootfs overlay and mkarchiso.".into(),
            "These backends are experimental previews until their output has been built and boot-tested on supported Linux hosts.".into(),
            "Local ISO creation currently requires a Linux host and the matching distribution build tools.".into(),
            "Theme and icon defaults are written to the GTK user profile; wallpaper assets are copied into the target filesystem. Some desktop environments may require additional session-specific wallpaper configuration.".into(),
        ],
    })
}

#[tauri::command]
pub fn prepare_os_build_project(workspace: String, project_toml: String) -> Result<BuildPreparation, String> {
    let root = super::workspace::canonical_root(&workspace)?;
    let project: OsProject = toml::from_str(&project_toml)
        .map_err(|error| format!("Invalid void-os.toml: {error}"))?;
    let report = validate_project(&project);
    if !report.valid {
        return Err(format!("Project validation failed: {}", report.errors.join("; ")));
    }
    match project.base.distribution.as_str() {
        "debian" => prepare_debian_live_project(workspace, project_toml),
        "ubuntu" | "arch" => prepare_non_debian_project(&root, &project),
        _ => Err("Unsupported Linux distribution.".into()),
    }
}
