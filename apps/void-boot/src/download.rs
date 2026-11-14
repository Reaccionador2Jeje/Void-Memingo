use serde::Serialize;
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex},
    thread,
};

#[derive(Debug, Clone, Serialize)]
pub struct DownloadSnapshot {
    pub id: String,
    pub file_name: String,
    pub path: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub status: String,
    pub error: Option<String>,
}

struct DownloadTask {
    snapshot: DownloadSnapshot,
    pause: Arc<AtomicBool>,
}

#[derive(Clone, Default)]
pub struct DownloadManager {
    tasks: Arc<Mutex<HashMap<String, DownloadTask>>>,
}

fn safe_file_name(value: &str) -> Result<String, String> {
    let name = Path::new(value).file_name().and_then(|v| v.to_str())
        .ok_or_else(|| "Nombre de archivo no válido".to_string())?;
    if name.is_empty() || name != value || name.contains("..")
        || !name.to_ascii_lowercase().ends_with(".iso")
        || !name.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
    {
        return Err("Usa un nombre simple terminado en .iso (letras, números, punto, guion o guion bajo)".into());
    }
    Ok(name.to_string())
}

impl DownloadManager {
    pub fn start(&self, url: String, file_name: String, directory: String) -> Result<String, String> {
        let parsed = reqwest::Url::parse(&url).map_err(|e| format!("URL inválida: {e}"))?;
        if parsed.scheme() != "https" || parsed.host_str().is_none() || parsed.username() != "" || parsed.password().is_some() {
            return Err("Solo se permiten enlaces HTTPS públicos sin credenciales".into());
        }
        let file_name = safe_file_name(&file_name)?;
        let dir = PathBuf::from(directory);
        if !dir.is_dir() { return Err("Selecciona una carpeta de destino existente".into()); }
        let final_path = dir.join(&file_name);
        let part_path = dir.join(format!("{file_name}.part"));
        if final_path.exists() { return Err("Ese archivo ya existe; elige otro nombre o mueve la ISO existente".into()); }
        let id = format!("{}-{}", file_name, dir.to_string_lossy());
        let pause = Arc::new(AtomicBool::new(false));
        {
            let mut tasks = self.tasks.lock().map_err(|_| "Gestor de descargas bloqueado".to_string())?;
            if tasks.get(&id).is_some_and(|task| task.snapshot.status == "downloading") {
                return Err("Esta descarga ya está en curso".into());
            }
            let existing = fs::metadata(&part_path).map(|m| m.len()).unwrap_or(0);
            tasks.insert(id.clone(), DownloadTask {
                snapshot: DownloadSnapshot {
                    id: id.clone(), file_name: file_name.clone(),
                    path: final_path.to_string_lossy().into_owned(),
                    downloaded_bytes: existing, total_bytes: None,
                    status: "queued".into(), error: None,
                },
                pause: pause.clone(),
            });
        }

        let tasks = self.tasks.clone();
        let id_for_thread = id.clone();
        thread::spawn(move || {
            let result = (|| -> Result<(), String> {
                let offset = fs::metadata(&part_path).map(|m| m.len()).unwrap_or(0);
                let client = reqwest::blocking::Client::builder()
                    .user_agent("VoidBoot-Downloader/0.1")
                    .timeout(std::time::Duration::from_secs(60))
                    .build().map_err(|e| e.to_string())?;
                let mut request = client.get(&url);
                if offset > 0 { request = request.header(reqwest::header::RANGE, format!("bytes={offset}-")); }
                let mut response = request.send().map_err(|e| format!("Error de red: {e}"))?;
                let status = response.status();
                if !(status.is_success()) { return Err(format!("El servidor respondió HTTP {status}")); }
                let append = offset > 0 && status == reqwest::StatusCode::PARTIAL_CONTENT;
                let start_at = if append { offset } else { 0 };
                let total = response.content_length().map(|n| n + start_at);
                let mut file = OpenOptions::new().create(true).write(true)
                    .append(append).truncate(!append).open(&part_path)
                    .map_err(|e| format!("No se pudo abrir el archivo temporal: {e}"))?;
                {
                    let mut map = tasks.lock().map_err(|_| "Gestor bloqueado".to_string())?;
                    if let Some(task) = map.get_mut(&id_for_thread) {
                        task.snapshot.status = "downloading".into();
                        task.snapshot.downloaded_bytes = start_at;
                        task.snapshot.total_bytes = total;
                    }
                }
                let mut buffer = vec![0u8; 256 * 1024];
                let mut downloaded = start_at;
                loop {
                    if pause.load(Ordering::Relaxed) { break; }
                    let count = response.read(&mut buffer).map_err(|e| format!("Descarga interrumpida: {e}"))?;
                    if count == 0 { break; }
                    file.write_all(&buffer[..count]).map_err(|e| format!("Error al guardar datos: {e}"))?;
                    downloaded += count as u64;
                    if let Ok(mut map) = tasks.lock() {
                        if let Some(task) = map.get_mut(&id_for_thread) {
                            task.snapshot.downloaded_bytes = downloaded;
                            task.snapshot.total_bytes = total;
                        }
                    }
                }
                file.flush().map_err(|e| e.to_string())?;
                if pause.load(Ordering::Relaxed) { return Ok(()); }
                if let Some(expected) = total {
                    if downloaded != expected { return Err(format!("Descarga incompleta: {downloaded} de {expected} bytes")); }
                }
                fs::rename(&part_path, &final_path).map_err(|e| format!("No se pudo finalizar el archivo: {e}"))?;
                Ok(())
            })();

            if let Ok(mut map) = tasks.lock() {
                if let Some(task) = map.get_mut(&id_for_thread) {
                    match result {
                        Ok(()) if pause.load(Ordering::Relaxed) => task.snapshot.status = "paused".into(),
                        Ok(()) => task.snapshot.status = "completed".into(),
                        Err(error) => { task.snapshot.status = "error".into(); task.snapshot.error = Some(error); }
                    }
                }
            }
        });
        Ok(id)
    }

    pub fn pause(&self, id: &str) -> Result<(), String> {
        let map = self.tasks.lock().map_err(|_| "Gestor de descargas bloqueado".to_string())?;
        let task = map.get(id).ok_or_else(|| "Descarga no encontrada".to_string())?;
        if task.snapshot.status != "downloading" && task.snapshot.status != "queued" {
            return Err("La descarga no está activa".into());
        }
        task.pause.store(true, Ordering::Relaxed);
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<DownloadSnapshot>, String> {
        let map = self.tasks.lock().map_err(|_| "Gestor de descargas bloqueado".to_string())?;
        Ok(map.values().map(|task| task.snapshot.clone()).collect())
    }
}
