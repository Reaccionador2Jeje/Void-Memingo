use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread,
};

const DOWNLOAD_CONNECTIONS: u64 = 8;
const BUFFER_SIZE: usize = 256 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct DownloadSnapshot {
    pub id: String,
    pub file_name: String,
    pub path: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub connections: u8,
    pub status: String,
    pub error: Option<String>,
    pub sha256: Option<String>,
    pub checksum_verified: bool,
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
    let name = Path::new(value)
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or_else(|| "Nombre de archivo no válido".to_string())?;
    if name.is_empty()
        || name != value
        || name.contains("..")
        || !name.to_ascii_lowercase().ends_with(".iso")
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
    {
        return Err("Usa un nombre simple terminado en .iso".into());
    }
    Ok(name.to_string())
}

fn normalize_expected_sha256(value: Option<String>) -> Result<Option<String>, String> {
    let normalized = value
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty());
    if normalized.as_ref().is_some_and(|value| {
        value.len() != 64 || !value.chars().all(|character| character.is_ascii_hexdigit())
    }) {
        return Err(
            "El SHA-256 esperado debe contener exactamente 64 caracteres hexadecimales".into(),
        );
    }
    Ok(normalized)
}

fn checksum_matches(actual: &str, expected: Option<&str>) -> bool {
    expected.is_none_or(|expected| actual.eq_ignore_ascii_case(expected))
}

fn update_progress(
    tasks: &Arc<Mutex<HashMap<String, DownloadTask>>>,
    id: &str,
    downloaded: u64,
    total: u64,
) {
    if let Ok(mut map) = tasks.lock() {
        if let Some(task) = map.get_mut(id) {
            task.snapshot.downloaded_bytes = downloaded;
            task.snapshot.total_bytes = Some(total);
        }
    }
}

impl DownloadManager {
    pub fn start(
        &self,
        url: String,
        file_name: String,
        directory: String,
        expected_sha256: Option<String>,
    ) -> Result<String, String> {
        let parsed = reqwest::Url::parse(&url).map_err(|e| format!("URL inválida: {e}"))?;
        if parsed.scheme() != "https"
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
        {
            return Err("Solo se permiten enlaces HTTPS públicos sin credenciales".into());
        }

        let expected_sha256 = normalize_expected_sha256(expected_sha256)?;

        let file_name = safe_file_name(&file_name)?;
        let directory = PathBuf::from(directory);
        if !directory.is_dir() {
            return Err("Selecciona una carpeta de destino existente".into());
        }
        let final_path = directory.join(&file_name);
        if final_path.exists() {
            return Err("Ese archivo ya existe; elige otro nombre".into());
        }

        let id = format!("{}-{}", file_name, directory.to_string_lossy());
        let pause = Arc::new(AtomicBool::new(false));
        {
            let mut map = self
                .tasks
                .lock()
                .map_err(|_| "Gestor bloqueado".to_string())?;
            if map
                .get(&id)
                .is_some_and(|t| t.snapshot.status == "downloading")
            {
                return Err("Esta descarga ya está en curso".into());
            }
            let partial = directory.join(format!("{file_name}.part"));
            let existing = fs::metadata(&partial).map_or(0, |m| m.len());
            map.insert(
                id.clone(),
                DownloadTask {
                    snapshot: DownloadSnapshot {
                        id: id.clone(),
                        file_name: file_name.clone(),
                        path: final_path.to_string_lossy().into_owned(),
                        downloaded_bytes: existing,
                        total_bytes: None,
                        connections: DOWNLOAD_CONNECTIONS as u8,
                        status: "queued".into(),
                        error: None,
                        sha256: None,
                        checksum_verified: false,
                    },
                    pause: pause.clone(),
                },
            );
        }

        let tasks = self.tasks.clone();
        let thread_id = id.clone();
        thread::spawn(move || {
            let result = (|| -> Result<(), String> {
                let expected_sha256 = expected_sha256.clone();
                let client = reqwest::blocking::Client::builder()
                    .user_agent("VoidBoot-Downloader/0.2")
                    .timeout(std::time::Duration::from_secs(60))
                    .build()
                    .map_err(|e| e.to_string())?;

                // Probe one byte to discover file size and range support before parallel transfer.
                let probe = client
                    .get(&url)
                    .header(reqwest::header::RANGE, "bytes=0-0")
                    .header(reqwest::header::ACCEPT_ENCODING, "identity")
                    .send()
                    .map_err(|e| format!("No se pudo consultar el servidor: {e}"))?;
                if probe.url().scheme() != "https" {
                    return Err("El servidor redirigió fuera de HTTPS; se cancela la descarga".into());
                }
                if probe.status() != reqwest::StatusCode::PARTIAL_CONTENT {
                    return Err("Este servidor no admite descargas por rangos. El modo acelerado requiere HTTP Range; no se inicia una descarga engañosamente paralela.".into());
                }
                let range = probe
                    .headers()
                    .get(reqwest::header::CONTENT_RANGE)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_string();
                let total = range
                    .rsplit('/')
                    .next()
                    .and_then(|v| v.parse::<u64>().ok())
                    .ok_or_else(|| {
                        "El servidor no informó el tamaño total de la ISO".to_string()
                    })?;
                drop(probe);
                if total == 0 {
                    return Err("El servidor informó un archivo vacío".into());
                }

                let part_paths: Vec<PathBuf> = (0..DOWNLOAD_CONNECTIONS)
                    .map(|i| directory.join(format!("{file_name}.part.{i:02}")))
                    .collect();
                let chunk_size = total.div_ceil(DOWNLOAD_CONNECTIONS);
                let progress = Arc::new(AtomicU64::new(
                    part_paths
                        .iter()
                        .map(|p| fs::metadata(p).map_or(0, |m| m.len()))
                        .sum(),
                ));
                update_progress(&tasks, &thread_id, progress.load(Ordering::Relaxed), total);
                if let Ok(mut map) = tasks.lock() {
                    if let Some(task) = map.get_mut(&thread_id) {
                        task.snapshot.status = "downloading".into();
                        task.snapshot.total_bytes = Some(total);
                    }
                }

                let mut workers = Vec::new();
                for index in 0..DOWNLOAD_CONNECTIONS {
                    let start = index * chunk_size;
                    if start >= total {
                        continue;
                    }
                    let end = std::cmp::min(start + chunk_size - 1, total - 1);
                    let chunk_path = part_paths[index as usize].clone();
                    let worker_client = client.clone();
                    let worker_url = url.clone();
                    let worker_pause = pause.clone();
                    let worker_progress = progress.clone();
                    let worker_tasks = tasks.clone();
                    let worker_id = thread_id.clone();

                    workers.push(thread::spawn(move || -> Result<(), String> {
                        let mut offset = fs::metadata(&chunk_path).map_or(0, |m| m.len());
                        let expected_len = end - start + 1;
                        if offset > expected_len {
                            fs::remove_file(&chunk_path).map_err(|e| e.to_string())?;
                            offset = 0;
                        }
                        if offset == expected_len {
                            return Ok(());
                        }

                        let request_start = start + offset;
                        let mut response = worker_client
                            .get(&worker_url)
                            .header(
                                reqwest::header::RANGE,
                                format!("bytes={request_start}-{end}"),
                            )
                            .header(reqwest::header::ACCEPT_ENCODING, "identity")
                            .send()
                            .map_err(|e| {
                                format!("Error de conexión en parte {}: {e}", index + 1)
                            })?;
                        if response.url().scheme() != "https" {
                            return Err("El servidor redirigió fuera de HTTPS durante la transferencia".into());
                        }
                        if response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
                            return Err(format!(
                                "El servidor dejó de admitir rangos en la parte {}",
                                index + 1
                            ));
                        }
                        let expected_header = format!("bytes {request_start}-{end}/{total}");
                        let actual_header = response
                            .headers()
                            .get(reqwest::header::CONTENT_RANGE)
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("");
                        if actual_header != expected_header {
                            return Err(format!(
                                "Rango incorrecto recibido en la parte {}",
                                index + 1
                            ));
                        }

                        let mut file = OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(&chunk_path)
                            .map_err(|e| {
                                format!("No se pudo guardar la parte {}: {e}", index + 1)
                            })?;
                        let mut buffer = vec![0u8; BUFFER_SIZE];
                        let mut received = offset;
                        while received < expected_len {
                            if worker_pause.load(Ordering::Relaxed) {
                                break;
                            }
                            let count = response.read(&mut buffer).map_err(|e| {
                                format!("Conexión interrumpida en parte {}: {e}", index + 1)
                            })?;
                            if count == 0 {
                                break;
                            }
                            let allowed =
                                std::cmp::min(count as u64, expected_len - received) as usize;
                            file.write_all(&buffer[..allowed])
                                .map_err(|e| e.to_string())?;
                            received += allowed as u64;
                            let now = worker_progress.fetch_add(allowed as u64, Ordering::Relaxed)
                                + allowed as u64;
                            update_progress(&worker_tasks, &worker_id, now, total);
                        }
                        file.flush().map_err(|e| e.to_string())?;
                        if !worker_pause.load(Ordering::Relaxed) && received != expected_len {
                            return Err(format!("La parte {} quedó incompleta", index + 1));
                        }
                        Ok(())
                    }));
                }

                let mut first_error = None;
                for worker in workers {
                    match worker.join() {
                        Ok(Ok(())) => {}
                        Ok(Err(error)) => {
                            if first_error.is_none() {
                                first_error = Some(error);
                            }
                        }
                        Err(_) => {
                            if first_error.is_none() {
                                first_error =
                                    Some("Una conexión de descarga terminó inesperadamente".into());
                            }
                        }
                    }
                }
                if let Some(error) = first_error {
                    return Err(error);
                }
                if pause.load(Ordering::Relaxed) {
                    return Ok(());
                }

                // Join the verified byte ranges in order into the final temporary ISO.
                let merged_path = directory.join(format!("{file_name}.part"));
                let mut merged = File::create(&merged_path).map_err(|e| e.to_string())?;
                for (index, path) in part_paths.iter().enumerate() {
                    let start = index as u64 * chunk_size;
                    if start >= total {
                        break;
                    }
                    let expected = std::cmp::min(chunk_size, total - start);
                    let actual = fs::metadata(path).map_err(|e| e.to_string())?.len();
                    if actual != expected {
                        return Err(format!(
                            "La parte {} no está completa: {actual}/{expected} bytes",
                            index + 1
                        ));
                    }
                    let mut part = File::open(path).map_err(|e| e.to_string())?;
                    std::io::copy(&mut part, &mut merged).map_err(|e| e.to_string())?;
                }
                merged.flush().map_err(|e| e.to_string())?;
                if fs::metadata(&merged_path).map_err(|e| e.to_string())?.len() != total {
                    return Err(
                        "El tamaño final no coincide con el tamaño anunciado por el servidor"
                            .into(),
                    );
                }
                let mut merged_file = File::open(&merged_path).map_err(|e| e.to_string())?;
                let mut hasher = Sha256::new();
                let mut hash_buffer = [0u8; BUFFER_SIZE];
                loop {
                    let count = merged_file
                        .read(&mut hash_buffer)
                        .map_err(|e| e.to_string())?;
                    if count == 0 {
                        break;
                    }
                    hasher.update(&hash_buffer[..count]);
                }
                let actual_sha256 = format!("{:x}", hasher.finalize());
                if let Some(expected) = expected_sha256.as_ref() {
                    if !checksum_matches(&actual_sha256, Some(expected)) {
                        let _ = fs::remove_file(&merged_path);
                        for path in &part_paths {
                            let _ = fs::remove_file(path);
                        }
                        return Err(format!("El SHA-256 no coincide. Esperado: {expected}; recibido: {actual_sha256}. Se descartaron las partes para evitar reanudar datos no verificados."));
                    }
                }
                fs::rename(&merged_path, &final_path)
                    .map_err(|e| format!("No se pudo finalizar la ISO: {e}"))?;
                if let Ok(mut map) = tasks.lock() {
                    if let Some(task) = map.get_mut(&thread_id) {
                        task.snapshot.sha256 = Some(actual_sha256);
                        task.snapshot.checksum_verified = expected_sha256.is_some();
                    }
                }
                for path in part_paths {
                    let _ = fs::remove_file(path);
                }
                Ok(())
            })();

            if let Ok(mut map) = tasks.lock() {
                if let Some(task) = map.get_mut(&thread_id) {
                    match result {
                        Ok(()) if pause.load(Ordering::Relaxed) => {
                            task.snapshot.status = "paused".into()
                        }
                        Ok(()) => task.snapshot.status = "completed".into(),
                        Err(error) => {
                            task.snapshot.status = "error".into();
                            task.snapshot.error = Some(error);
                        }
                    }
                }
            }
        });
        Ok(id)
    }

    pub fn pause(&self, id: &str) -> Result<(), String> {
        let map = self
            .tasks
            .lock()
            .map_err(|_| "Gestor bloqueado".to_string())?;
        let task = map
            .get(id)
            .ok_or_else(|| "Descarga no encontrada".to_string())?;
        if task.snapshot.status != "downloading" && task.snapshot.status != "queued" {
            return Err("La descarga no está activa".into());
        }
        task.pause.store(true, Ordering::Relaxed);
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<DownloadSnapshot>, String> {
        let map = self
            .tasks
            .lock()
            .map_err(|_| "Gestor bloqueado".to_string())?;
        Ok(map.values().map(|task| task.snapshot.clone()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::safe_file_name;

    #[test]
    fn accepts_simple_iso_file_names() {
        assert_eq!(
            safe_file_name("debian-live.iso").unwrap(),
            "debian-live.iso"
        );
        assert_eq!(
            safe_file_name("void_boot_01.iso").unwrap(),
            "void_boot_01.iso"
        );
    }

    #[test]
    fn normalizes_and_validates_publisher_sha256() {
        let uppercase = "A".repeat(64);
        assert_eq!(
            normalize_expected_sha256(Some(uppercase)).unwrap(),
            Some("a".repeat(64))
        );
        assert_eq!(normalize_expected_sha256(None).unwrap(), None);
        assert!(normalize_expected_sha256(Some("not-a-checksum".into())).is_err());
        assert!(normalize_expected_sha256(Some("a".repeat(63))).is_err());
    }

    #[test]
    fn publisher_checksum_must_match_before_finalization() {
        let expected = "a".repeat(64);
        assert!(checksum_matches(&expected, Some(&expected)));
        assert!(!checksum_matches(&"b".repeat(64), Some(&expected)));
        assert!(checksum_matches("any-digest", None));
    }

    #[test]
    fn rejects_paths_and_non_iso_names() {
        assert!(safe_file_name("../debian.iso").is_err());
        assert!(safe_file_name("folder/debian.iso").is_err());
        assert!(safe_file_name("debian.zip").is_err());
        assert!(safe_file_name("debian.iso/other.iso").is_err());
    }
}
