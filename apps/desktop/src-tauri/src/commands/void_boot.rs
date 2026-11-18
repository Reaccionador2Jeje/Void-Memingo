use std::path::Path;
use ::void_boot::{catalog::{load_distributions, Distribution}, profiles::ProfileCatalog, scan_directory, IsoEntry};

#[tauri::command]
pub fn list_void_boot_distributions() -> Result<Vec<Distribution>, String> {
    load_distributions().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn scan_void_boot_iso_directory(directory: String) -> Result<Vec<IsoEntry>, String> {
    scan_directory(Path::new(&directory)).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_void_boot_release_gate() -> ::void_boot::releases::DistroCenterReleaseGate {
    ::void_boot::releases::distro_center_release_gate()
}

#[tauri::command]
pub fn get_void_boot_mirror_support(distribution_id: String) -> Result<::void_boot::releases::MirrorSupport, String> {
    ::void_boot::releases::mirror_support_for_distribution(&distribution_id)
}

#[tauri::command]
pub fn list_void_boot_arch_official_mirrors() -> Result<Vec<::void_boot::releases::OfficialMirror>, String> {
    ::void_boot::releases::list_arch_official_mirrors()
}

#[tauri::command]
pub fn list_void_boot_kali_official_download_candidates(
    artifact_path: String,
) -> Result<Vec<::void_boot::releases::KaliDownloadCandidate>, String> {
    ::void_boot::releases::list_kali_official_download_candidates(&artifact_path)
}

#[tauri::command]
pub fn list_void_boot_debian_official_download_candidates(
    artifact_path: String,
) -> Result<Vec<::void_boot::releases::DebianDownloadCandidate>, String> {
    ::void_boot::releases::list_debian_official_download_candidates(&artifact_path)
}

#[tauri::command]
pub fn list_void_boot_fedora_official_download_candidates(
    release: String,
    edition: String,
    architecture: String,
    file_name: String,
) -> Result<Vec<::void_boot::releases::OfficialPathDownloadCandidate>, String> {
    ::void_boot::releases::list_fedora_official_download_candidates(
        &release, &edition, &architecture, &file_name,
    )
}

#[tauri::command]
pub fn list_void_boot_ubuntu_official_download_candidates(
    distribution_id: String,
    release: String,
    file_name: String,
) -> Result<Vec<::void_boot::releases::OfficialPathDownloadCandidate>, String> {
    ::void_boot::releases::list_ubuntu_official_download_candidates(
        &distribution_id, &release, &file_name,
    )
}

#[tauri::command]
pub fn list_void_boot_opensuse_official_download_candidates(
    artifact_path: String,
) -> Result<Vec<::void_boot::releases::OfficialPathDownloadCandidate>, String> {
    ::void_boot::releases::list_opensuse_official_download_candidates(&artifact_path)
}

#[tauri::command]
pub fn list_void_boot_linux_mint_official_download_candidates(
    artifact_path: String,
) -> Result<Vec<::void_boot::releases::OfficialPathDownloadCandidate>, String> {
    ::void_boot::releases::list_linux_mint_official_download_candidates(&artifact_path)
}

#[tauri::command]
pub fn list_void_boot_freebsd_official_download_candidates(
    artifact_path: String,
) -> Result<Vec<::void_boot::releases::OfficialPathDownloadCandidate>, String> {
    ::void_boot::releases::list_freebsd_official_download_candidates(&artifact_path)
}

#[tauri::command]
pub fn list_void_boot_manjaro_official_download_candidates(
    edition: String,
    release: String,
    file_name: String,
) -> Result<Vec<::void_boot::releases::OfficialPathDownloadCandidate>, String> {
    ::void_boot::releases::list_manjaro_official_download_candidates(
        &edition,
        &release,
        &file_name,
    )
}

#[tauri::command]
#[tauri::command]
pub fn list_void_boot_zorin_official_download_candidates(
    release: String,
    file_name: String,
) -> Result<Vec<::void_boot::releases::OfficialPathDownloadCandidate>, String> {
    ::void_boot::releases::list_zorin_official_download_candidates(&release, &file_name)
}

pub fn list_void_boot_endeavouros_official_download_candidates(
    file_name: String,
) -> Result<Vec<::void_boot::releases::OfficialPathDownloadCandidate>, String> {
    ::void_boot::releases::list_endeavouros_official_download_candidates(&file_name)
}

#[tauri::command]
pub fn list_void_boot_centos_stream_official_download_candidates(
    artifact_path: String,
) -> Result<Vec<::void_boot::releases::OfficialPathDownloadCandidate>, String> {
    ::void_boot::releases::list_centos_stream_official_download_candidates(&artifact_path)
}

#[tauri::command]
pub fn list_void_boot_oracle_linux_official_download_candidates(
    artifact_path: String,
) -> Result<Vec<::void_boot::releases::OfficialPathDownloadCandidate>, String> {
    ::void_boot::releases::list_oracle_linux_official_download_candidates(&artifact_path)
}

#[tauri::command]
pub fn list_void_boot_rocky_official_download_candidates(
    artifact_path: String,
) -> Result<Vec<::void_boot::releases::OfficialPathDownloadCandidate>, String> {
    ::void_boot::releases::list_rocky_official_download_candidates(&artifact_path)
}

#[tauri::command]
pub fn list_void_boot_almalinux_official_download_candidates(
    artifact_path: String,
) -> Result<Vec<::void_boot::releases::OfficialPathDownloadCandidate>, String> {
    ::void_boot::releases::list_almalinux_official_download_candidates(&artifact_path)
}

#[tauri::command]
pub fn list_void_boot_openbsd_official_download_candidates(
    artifact_path: String,
) -> Result<Vec<::void_boot::releases::OfficialPathDownloadCandidate>, String> {
    ::void_boot::releases::list_openbsd_official_download_candidates(&artifact_path)
}

#[tauri::command]
pub fn list_void_boot_netbsd_official_download_candidates(
    artifact_path: String,
) -> Result<Vec<::void_boot::releases::OfficialPathDownloadCandidate>, String> {
    ::void_boot::releases::list_netbsd_official_download_candidates(&artifact_path)
}

#[tauri::command]
pub fn lookup_void_boot_official_release_checksum(
    distribution_id: String,
    file_name: String,
) -> Result<::void_boot::releases::OfficialReleaseChecksum, String> {
    ::void_boot::releases::lookup_official_release_checksum(&distribution_id, &file_name)
}

#[tauri::command]
pub fn read_void_boot_expected_sha256(
    manifest_path: String,
    file_name: String,
) -> Result<String, String> {
    if file_name.is_empty()
        || file_name != Path::new(&file_name).file_name().and_then(|name| name.to_str()).unwrap_or_default()
        || !file_name.to_ascii_lowercase().ends_with(".iso")
        || !file_name.chars().all(|character| character.is_ascii_alphanumeric() || "._-".contains(character))
    {
        return Err("Usa un nombre simple terminado en .iso para consultar el manifiesto.".into());
    }
    ::void_boot::expected_sha256_from_manifest(Path::new(&manifest_path), &file_name)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn verify_void_boot_iso_checksum_manifest(
    iso_path: String,
    manifest_path: String,
) -> Result<(), String> {
    ::void_boot::verify_iso_sha256_manifest(Path::new(&iso_path), Path::new(&manifest_path))
        .map_err(|error| error.to_string())
}


#[tauri::command]
pub fn load_void_boot_profile_catalog(profile_file: String) -> Result<Vec<::void_boot::profiles::BootProfile>, String> {
    let catalog = ProfileCatalog::load(Path::new(&profile_file))
        .map_err(|error| error.to_string())?;
    Ok(catalog.profiles)
}

#[tauri::command]
pub fn save_void_boot_profile_catalog(
    profile_file: String,
    profiles: Vec<::void_boot::profiles::BootProfile>,
) -> Result<(), String> {
    use std::io::Write;

    let destination = Path::new(&profile_file);
    if destination.extension().and_then(|value| value.to_str()) != Some("toml") {
        return Err("El catálogo debe guardarse como archivo .toml.".to_string());
    }
    let parent = destination.parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if !parent.is_dir() {
        return Err("La carpeta de destino no existe.".to_string());
    }
    if std::fs::symlink_metadata(destination)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err("No se permite sobrescribir un enlace simbólico.".to_string());
    }

    let catalog = ProfileCatalog { profiles };
    catalog.validate().map_err(|error| error.to_string())?;
    let serialized = toml::to_string_pretty(&catalog)
        .map_err(|error| error.to_string())?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(destination)
        .map_err(|error| error.to_string())?;
    file.write_all(serialized.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn preview_void_boot_menu(profile_file: String) -> Result<String, String> {
    let catalog = ProfileCatalog::load(Path::new(&profile_file))
        .map_err(|error| error.to_string())?;
    Ok(catalog.render_grub_config())
}

#[tauri::command]
pub fn start_void_boot_download(
    manager: tauri::State<'_, ::void_boot::download::DownloadManager>,
    distribution_id: String,
    url: String,
    file_name: String,
    directory: String,
    expected_sha256: Option<String>,
) -> Result<String, String> {
    ::void_boot::releases::ensure_distro_center_downloads_enabled(
        &distribution_id,
        &file_name,
        expected_sha256.as_deref(),
    )?;
    manager.start(url, file_name, directory, expected_sha256)
}

#[tauri::command]
pub fn pause_void_boot_download(
    manager: tauri::State<'_, ::void_boot::download::DownloadManager>,
    id: String,
) -> Result<(), String> {
    manager.pause(&id)
}

#[tauri::command]
pub fn list_void_boot_downloads(
    manager: tauri::State<'_, ::void_boot::download::DownloadManager>,
) -> Result<Vec<::void_boot::download::DownloadSnapshot>, String> {
    manager.list()
}


#[tauri::command]
pub fn start_void_boot_qemu(image_path: String) -> Result<u32, String> {
    use std::process::{Command, Stdio};

    let image = Path::new(&image_path);
    if !image.is_file() || image.metadata().map_err(|e| e.to_string())?.len() == 0 {
        return Err("La imagen virtual no existe o está vacía.".to_string());
    }
    if !cfg!(target_os = "linux") {
        return Err("El lanzador QEMU integrado está disponible actualmente en Linux.".to_string());
    }

    let qemu = Command::new("qemu-system-x86_64")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| "No se encontró qemu-system-x86_64. Instala QEMU antes de iniciar la máquina.".to_string())?;
    if !qemu.success() {
        return Err("QEMU no pudo ejecutarse correctamente.".to_string());
    }

    let firmware_pairs = [
        ("/usr/share/OVMF/OVMF_CODE_4M.fd", "/usr/share/OVMF/OVMF_VARS_4M.fd"),
        ("/usr/share/OVMF/OVMF_CODE.fd", "/usr/share/OVMF/OVMF_VARS.fd"),
        ("/usr/share/edk2/ovmf/OVMF_CODE_4M.fd", "/usr/share/edk2/ovmf/OVMF_VARS_4M.fd"),
        ("/usr/share/edk2/ovmf/OVMF_CODE.fd", "/usr/share/edk2/ovmf/OVMF_VARS.fd"),
    ];
    let (code, vars) = firmware_pairs.iter()
        .find(|(code, vars)| Path::new(code).is_file() && Path::new(vars).is_file())
        .ok_or_else(|| "No se encontró OVMF. Instala el paquete ovmf.".to_string())?;

    let vars_copy = std::env::temp_dir().join(format!("void-boot-ovmf-vars-{}.fd", std::process::id()));
    let image_format = match image.extension().and_then(|value| value.to_str()).unwrap_or("").to_ascii_lowercase().as_str() {
        "qcow2" => "qcow2",
        _ => "raw",
    };
    std::fs::copy(vars, &vars_copy).map_err(|e| format!("No se pudo preparar OVMF: {e}"))?;

    let child = Command::new("qemu-system-x86_64")
        .arg("-machine").arg("q35,accel=tcg")
        .arg("-cpu").arg("max")
        .arg("-m").arg("2048")
        .arg("-drive").arg(format!("if=pflash,format=raw,readonly=on,file={code}"))
        .arg("-drive").arg(format!("if=pflash,format=raw,file={}", vars_copy.display()))
        .arg("-drive").arg(format!("file={},format={},if=virtio,snapshot=on", image.display(), image_format))
        .arg("-boot").arg("order=c")
        .arg("-net").arg("none")
        .arg("-no-reboot")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("No se pudo iniciar QEMU: {e}"))?;

    Ok(child.id())
}


#[tauri::command]
pub fn create_void_boot_qemu_disk(image_path: String, size: String) -> Result<String, String> {
    use std::process::{Command, Stdio};

    if !cfg!(target_os = "linux") {
        return Err("La creación integrada de discos QEMU está disponible actualmente en Linux.".to_string());
    }
    let destination = Path::new(&image_path);
    if destination.exists() {
        return Err("El archivo de destino ya existe; elige otro nombre para evitar sobrescribir datos.".to_string());
    }
    if size != "8G" && size != "16G" && size != "32G" {
        return Err("Tamaño no permitido. Usa 8G, 16G o 32G.".to_string());
    }
    let result = Command::new("qemu-img")
        .arg("create").arg("-f").arg("qcow2")
        .arg(&image_path).arg(&size)
        .stdout(Stdio::piped()).stderr(Stdio::piped())
        .output()
        .map_err(|_| "No se encontró qemu-img. Instala el paquete qemu-utils.".to_string())?;
    if !result.status.success() {
        return Err(String::from_utf8_lossy(&result.stderr).trim().to_string());
    }
    Ok(image_path)
}
