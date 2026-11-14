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
