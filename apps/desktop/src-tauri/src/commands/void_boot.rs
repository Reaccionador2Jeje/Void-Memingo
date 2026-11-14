use std::path::Path;
use void_boot::{scan_directory, IsoEntry};

#[tauri::command]
pub fn scan_void_boot_iso_directory(directory: String) -> Result<Vec<IsoEntry>, String> {
    scan_directory(Path::new(&directory)).map_err(|error| error.to_string())
}
