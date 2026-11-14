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
