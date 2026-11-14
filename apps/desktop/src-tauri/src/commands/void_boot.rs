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
pub fn preview_void_boot_menu(profile_file: String) -> Result<String, String> {
    let catalog = ProfileCatalog::load(Path::new(&profile_file))
        .map_err(|error| error.to_string())?;
    Ok(catalog.render_grub_config())
}

#[tauri::command]
pub fn start_void_boot_download(
    manager: tauri::State<'_, ::void_boot::download::DownloadManager>,
    url: String,
    file_name: String,
    directory: String,
) -> Result<String, String> {
    manager.start(url, file_name, directory)
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
