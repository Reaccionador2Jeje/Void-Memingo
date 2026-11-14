use serde::Serialize;
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    name: String,
    path: String,
    is_dir: bool,
    size: Option<u64>,
}

pub(super) fn canonical_root(root: &str) -> Result<PathBuf, String> {
    let path = fs::canonicalize(root)
        .map_err(|error| format!("Cannot open workspace: {error}"))?;
    if !path.is_dir() {
        return Err("The selected workspace is not a directory.".into());
    }
    Ok(path)
}

fn safe_relative_path(relative: &str) -> Result<PathBuf, String> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path.components().any(|part| {
            matches!(
                part,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err("The requested path is outside the workspace.".into());
    }
    Ok(path.to_path_buf())
}

fn resolve_existing(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let relative = safe_relative_path(relative)?;
    let candidate = fs::canonicalize(root.join(relative))
        .map_err(|error| format!("Cannot access workspace item: {error}"))?;
    if !candidate.starts_with(root) {
        return Err("Access outside the workspace is blocked.".into());
    }
    Ok(candidate)
}

fn resolve_file_for_write(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let relative = safe_relative_path(relative)?;
    if relative.as_os_str().is_empty() || relative.file_name().is_none() {
        return Err("A file name is required.".into());
    }

    let candidate = root.join(&relative);
    let parent = candidate
        .parent()
        .ok_or_else(|| "The file must be inside the workspace.".to_string())?;
    let canonical_parent = fs::canonicalize(parent)
        .map_err(|error| format!("Cannot access the target directory: {error}"))?;

    if !canonical_parent.starts_with(root) {
        return Err("Writing outside the workspace is blocked.".into());
    }

    if candidate.exists() {
        let canonical_target = fs::canonicalize(&candidate)
            .map_err(|error| format!("Cannot access target file: {error}"))?;
        if !canonical_target.starts_with(root) || !canonical_target.is_file() {
            return Err("The target is not a regular file inside the workspace.".into());
        }
        return Ok(canonical_target);
    }

    Ok(canonical_parent.join(
        candidate
            .file_name()
            .ok_or_else(|| "A valid file name is required.".to_string())?,
    ))
}

#[tauri::command]
pub fn app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[tauri::command]
pub fn list_directory(root: String, relative_path: String) -> Result<Vec<FileEntry>, String> {
    let root = canonical_root(&root)?;
    let directory = if relative_path.is_empty() {
        root.clone()
    } else {
        resolve_existing(&root, &relative_path)?
    };

    if !directory.is_dir() {
        return Err("The selected workspace item is not a directory.".into());
    }

    let mut entries = Vec::new();
    let items = fs::read_dir(&directory)
        .map_err(|error| format!("Cannot read directory: {error}"))?;

    for item in items {
        let item = item.map_err(|error| format!("Cannot read directory entry: {error}"))?;
        let file_type = item
            .file_type()
            .map_err(|error| format!("Cannot inspect directory entry: {error}"))?;

        // Do not follow symlinks: this keeps workspace access inside the selected root.
        if file_type.is_symlink() || (!file_type.is_dir() && !file_type.is_file()) {
            continue;
        }

        let path = item.path();
        let relative = path
            .strip_prefix(&root)
            .map_err(|_| "A directory entry escaped the workspace.".to_string())?;
        let metadata = item
            .metadata()
            .map_err(|error| format!("Cannot inspect file metadata: {error}"))?;

        entries.push(FileEntry {
            name: item.file_name().to_string_lossy().into_owned(),
            path: relative.to_string_lossy().replace('\\', "/"),
            is_dir: file_type.is_dir(),
            size: if file_type.is_file() {
                Some(metadata.len())
            } else {
                None
            },
        });
    }

    entries.sort_by(|left, right| {
        right
            .is_dir
            .cmp(&left.is_dir)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    Ok(entries)
}

#[tauri::command]
pub fn read_text_file(root: String, relative_path: String) -> Result<String, String> {
    let root = canonical_root(&root)?;
    let path = resolve_existing(&root, &relative_path)?;
    if !path.is_file() {
        return Err("The selected item is not a file.".into());
    }
    fs::read_to_string(path).map_err(|error| {
        format!("Cannot read this file as UTF-8 text: {error}")
    })
}

#[tauri::command]
pub fn write_text_file(
    root: String,
    relative_path: String,
    contents: String,
) -> Result<(), String> {
    let root = canonical_root(&root)?;
    let path = resolve_file_for_write(&root, &relative_path)?;
    fs::write(path, contents).map_err(|error| format!("Cannot save file: {error}"))
}



#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchMatch {
    path: String,
    line_number: usize,
    column: usize,
    preview: String,
}

fn ignored_search_directory(name: &str) -> bool {
    matches!(
        name,
        ".git" | "node_modules" | "target" | "dist" | "build" | "out"
            | ".void-memingo" | ".venv" | "venv" | "vendor" | ".cache"
    )
}

fn search_workspace_files(
    root: &Path,
    query: &str,
    case_sensitive: bool,
) -> Result<Vec<SearchMatch>, String> {
    const MAX_FILES: usize = 20_000;
    const MAX_FILE_BYTES: u64 = 1_048_576;
    const MAX_MATCHES: usize = 1_000;

    let needle = if case_sensitive { query.to_string() } else { query.to_lowercase() };
    let mut pending = vec![root.to_path_buf()];
    let mut matches = Vec::new();
    let mut scanned = 0usize;

    while let Some(directory) = pending.pop() {
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            if matches.len() >= MAX_MATCHES || scanned >= MAX_FILES {
                return Ok(matches);
            }
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(_) => continue,
            };
            if file_type.is_symlink() {
                continue;
            }
            let path = entry.path();
            if file_type.is_dir() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if !ignored_search_directory(&name) {
                    pending.push(path);
                }
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };
            if metadata.len() > MAX_FILE_BYTES {
                continue;
            }
            scanned += 1;
            let bytes = match fs::read(&path) {
                Ok(bytes) => bytes,
                Err(_) => continue,
            };
            if bytes.contains(&0) {
                continue;
            }
            let contents = match String::from_utf8(bytes) {
                Ok(contents) => contents,
                Err(_) => continue,
            };
            let relative = match path.strip_prefix(root) {
                Ok(relative) => relative.to_string_lossy().replace('\\', "/"),
                Err(_) => continue,
            };
            for (line_index, line) in contents.lines().enumerate() {
                let haystack = if case_sensitive { line.to_string() } else { line.to_lowercase() };
                if let Some(byte_index) = haystack.find(&needle) {
                    let column = haystack[..byte_index].chars().count() + 1;
                    let mut preview: String = line.trim().chars().take(240).collect();
                    if line.trim().chars().count() > 240 {
                        preview.push('…');
                    }
                    matches.push(SearchMatch {
                        path: relative.clone(),
                        line_number: line_index + 1,
                        column,
                        preview,
                    });
                    if matches.len() >= MAX_MATCHES {
                        return Ok(matches);
                    }
                }
            }
        }
    }
    Ok(matches)
}

#[tauri::command]
pub async fn search_workspace(
    root: String,
    query: String,
    case_sensitive: bool,
) -> Result<Vec<SearchMatch>, String> {
    let root = canonical_root(&root)?;
    let query = query.trim().to_string();
    if query.is_empty() {
        return Err("Escribe un texto para buscar.".into());
    }
    if query.len() > 256 {
        return Err("La búsqueda no puede superar 256 bytes.".into());
    }
    tauri::async_runtime::spawn_blocking(move || search_workspace_files(&root, &query, case_sensitive))
        .await
        .map_err(|error| format!("Workspace search worker failed: {error}"))?
}

#[cfg(test)]
mod tests {
    use super::safe_relative_path;

    #[test]
    fn rejects_parent_directory_traversal() {
        assert!(safe_relative_path("../outside.txt").is_err());
        assert!(safe_relative_path("src/../../outside.txt").is_err());
    }

    #[test]
    fn rejects_absolute_paths() {
        assert!(safe_relative_path("/etc/passwd").is_err());
    }

    #[test]
    fn accepts_workspace_relative_paths() {
        assert!(safe_relative_path("src/main.void").is_ok());
        assert!(safe_relative_path("README.md").is_ok());
    }
}


#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalResult {
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
    success: bool,
}

fn truncate_output(value: String) -> String {
    const MAX_OUTPUT_BYTES: usize = 1024 * 1024;
    if value.len() <= MAX_OUTPUT_BYTES {
        return value;
    }
    let mut end = MAX_OUTPUT_BYTES;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}\n\n[Output truncated at 1 MiB]", &value[..end])
}

pub(super) fn terminal_directory(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let safe = safe_relative_path(relative)?;
    let path = fs::canonicalize(root.join(safe))
        .map_err(|error| format!("Cannot access terminal directory: {error}"))?;
    if !path.starts_with(root) {
        return Err("Terminal directory must stay inside the workspace.".into());
    }
    if !path.is_dir() {
        return Err("Terminal working directory is not a directory.".into());
    }
    Ok(path)
}

#[tauri::command]
pub fn resolve_terminal_directory(
    root: String,
    current_directory: String,
    target: String,
) -> Result<String, String> {
    let root = canonical_root(&root)?;
    let current = terminal_directory(&root, &current_directory)?;
    let target_path = Path::new(target.trim());
    let joined = if target_path.is_absolute() {
        target_path.to_path_buf()
    } else {
        current.join(target_path)
    };
    let canonical = fs::canonicalize(&joined)
        .map_err(|error| format!("Cannot change directory: {error}"))?;
    if !canonical.starts_with(&root) {
        return Err("Changing directory outside the workspace is blocked.".into());
    }
    if !canonical.is_dir() {
        return Err("The requested path is not a directory.".into());
    }
    let relative = canonical
        .strip_prefix(&root)
        .map_err(|_| "Terminal directory escaped the workspace.".to_string())?;
    Ok(relative.to_string_lossy().replace('\\', "/"))
}

#[tauri::command]
pub async fn run_terminal_command(
    root: String,
    current_directory: String,
    command: String,
) -> Result<TerminalResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = canonical_root(&root)?;
        let cwd = terminal_directory(&root, &current_directory)?;
        let command = command.trim();
        if command.is_empty() {
            return Err("Enter a command first.".into());
        }
        if command.len() > 8192 {
            return Err("Command is too long (maximum 8192 bytes).".into());
        }

        #[cfg(windows)]
        let output = std::process::Command::new("cmd")
            .args(["/C", command])
            .current_dir(&cwd)
            .output();

        #[cfg(not(windows))]
        let output = std::process::Command::new("sh")
            .args(["-lc", command])
            .current_dir(&cwd)
            .output();

        let output = output.map_err(|error| format!("Could not start command: {error}"))?;
        let stdout = truncate_output(String::from_utf8_lossy(&output.stdout).into_owned());
        let stderr = truncate_output(String::from_utf8_lossy(&output.stderr).into_owned());
        Ok(TerminalResult {
            stdout,
            stderr,
            exit_code: output.status.code(),
            success: output.status.success(),
        })
    })
    .await
    .map_err(|error| format!("Terminal worker failed: {error}"))?
}
