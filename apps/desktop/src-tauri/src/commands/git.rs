use serde::Serialize;
use std::{fs, path::Path, process::Command};

use super::workspace::canonical_root;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRemote {
    name: String,
    url: String,
    provider: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitSnapshot {
    initialized: bool,
    branch: String,
    status: String,
    staged_summary: String,
    remotes: Vec<GitRemote>,
    branches: Vec<String>,
    recent_commits: Vec<String>,
}

fn run_git(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|error| format!("Git is not installed or could not start: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if stderr.is_empty() { stdout } else { stderr };
        return Err(if detail.is_empty() {
            format!("Git command failed with status {}.", output.status)
        } else {
            detail
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn provider_for_remote(url: &str) -> String {
    let normalized = url.to_ascii_lowercase();
    if normalized.contains("github.com") {
        "GitHub".into()
    } else if normalized.contains("gitlab.com") || normalized.contains("gitlab.") {
        "GitLab".into()
    } else if normalized.contains("gitvoid") {
        "GitVoid (experimental)".into()
    } else {
        "Git remoto".into()
    }
}

fn snapshot(root: &Path) -> Result<GitSnapshot, String> {
    let probe = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map_err(|error| format!("Git is not installed or could not start: {error}"))?;

    if !probe.status.success() {
        return Ok(GitSnapshot {
            initialized: false,
            branch: String::new(),
            status: String::new(),
            staged_summary: String::new(),
            remotes: Vec::new(),
            branches: Vec::new(),
            recent_commits: Vec::new(),
        });
    }

    let branch = run_git(root, &["branch", "--show-current"])?;
    let status = run_git(root, &["status", "--short", "--branch"])?;
    let staged_summary = run_git(root, &["diff", "--cached", "--stat"])?;
    let remote_output = run_git(root, &["remote", "-v"])?;
    let mut remotes: Vec<GitRemote> = Vec::new();

    for line in remote_output.lines() {
        let mut fields = line.split_whitespace();
        let Some(name) = fields.next() else { continue };
        let Some(url) = fields.next() else { continue };
        let direction = fields.next().unwrap_or("");
        if direction != "(fetch)" || remotes.iter().any(|remote| remote.name == name) {
            continue;
        }
        remotes.push(GitRemote {
            name: name.to_string(),
            url: url.to_string(),
            provider: provider_for_remote(url),
        });
    }

    let branches = run_git(root, &["branch", "--format=%(refname:short)"])?
        .lines().map(str::to_string).collect();
    let recent_commits = run_git(root, &["log", "-5", "--format=%h%x09%s"])
        .unwrap_or_default()
        .lines().map(str::to_string).collect();

    Ok(GitSnapshot {
        initialized: true,
        branch,
        status,
        staged_summary,
        remotes,
        branches,
        recent_commits,
    })
}

#[tauri::command]
pub async fn git_clone_repository(
    destination_parent: String,
    url: String,
    folder_name: String,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let parent = canonical_root(&destination_parent)?;
        let url = url.trim();
        let valid_scheme = ["https://", "ssh://", "git://"].iter().any(|scheme| url.starts_with(scheme))
            || url.starts_with("git@");
        if !valid_scheme || url.len() > 2048 || url.chars().any(char::is_whitespace) {
            return Err("Use a valid HTTPS, SSH or Git remote URL.".into());
        }

        let folder = folder_name.trim();
        if folder.is_empty()
            || folder.len() > 100
            || folder.starts_with('.')
            || !folder.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        {
            return Err("Folder name must be a simple name using letters, numbers, dots, underscores or hyphens.".into());
        }
        let destination = parent.join(folder);
        if destination.exists() {
            return Err("The destination folder already exists.".into());
        }

        let output = Command::new("git")
            .arg("clone")
            .arg("--")
            .arg(url)
            .arg(&destination)
            .output()
            .map_err(|error| format!("Git is not installed or could not start: {error}"))?;
        if !output.status.success() {
            let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if detail.is_empty() { "Git clone failed.".into() } else { detail });
        }
        fs::canonicalize(&destination)
            .map(|path| path.to_string_lossy().into_owned())
            .map_err(|error| format!("Repository cloned, but its folder could not be resolved: {error}"))
    })
    .await
    .map_err(|error| format!("Git clone worker failed: {error}"))?
}

#[tauri::command]
pub fn git_snapshot(root: String) -> Result<GitSnapshot, String> {
    let root = canonical_root(&root)?;
    snapshot(&root)
}

#[tauri::command]
pub fn git_init(root: String) -> Result<GitSnapshot, String> {
    let root = canonical_root(&root)?;
    run_git(&root, &["init"])?;
    snapshot(&root)
}

#[tauri::command]
pub fn git_stage_all(root: String) -> Result<GitSnapshot, String> {
    let root = canonical_root(&root)?;
    run_git(&root, &["add", "--all"])?;
    snapshot(&root)
}

#[tauri::command]
pub fn git_commit(root: String, message: String) -> Result<GitSnapshot, String> {
    let root = canonical_root(&root)?;
    let message = message.trim();
    if message.is_empty() {
        return Err("Commit message cannot be empty.".into());
    }
    if message.len() > 512 {
        return Err("Commit message is too long (maximum 512 bytes).".into());
    }
    run_git(&root, &["commit", "-m", message])?;
    snapshot(&root)
}

#[tauri::command]
pub fn git_add_remote(root: String, name: String, url: String) -> Result<GitSnapshot, String> {
    let root = canonical_root(&root)?;
    let name = name.trim();
    let url = url.trim();

    if name.is_empty()
        || name.len() > 64
        || !name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        || name.starts_with('-')
    {
        return Err("Remote name must use only letters, numbers, dots, underscores or hyphens.".into());
    }
    let valid_scheme = ["https://", "ssh://", "git://"].iter().any(|scheme| url.starts_with(scheme))
        || url.starts_with("git@");
    if !valid_scheme || url.len() > 2048 || url.chars().any(char::is_whitespace) {
        return Err("Use a valid HTTPS, SSH or Git remote URL.".into());
    }

    run_git(&root, &["remote", "add", name, url])?;
    snapshot(&root)
}

#[tauri::command]
pub fn git_create_branch(root: String, branch: String) -> Result<GitSnapshot, String> {
    let root = canonical_root(&root)?;
    let branch = branch.trim();
    if branch.is_empty() || branch.len() > 200 || branch.starts_with('-') {
        return Err("Enter a valid branch name.".into());
    }
    run_git(&root, &["check-ref-format", "--branch", branch])?;
    run_git(&root, &["switch", "-c", branch])?;
    snapshot(&root)
}

#[tauri::command]
pub fn git_checkout_branch(root: String, branch: String) -> Result<GitSnapshot, String> {
    let root = canonical_root(&root)?;
    let branch = branch.trim();
    if branch.is_empty() || branch.len() > 200 || branch.starts_with('-') {
        return Err("Enter a valid branch name.".into());
    }
    run_git(&root, &["check-ref-format", "--branch", branch])?;
    run_git(&root, &["switch", branch])?;
    snapshot(&root)
}

#[tauri::command]
pub fn git_fetch_all(root: String) -> Result<GitSnapshot, String> {
    let root = canonical_root(&root)?;
    run_git(&root, &["fetch", "--all", "--prune"])?;
    snapshot(&root)
}


#[tauri::command]
pub fn git_pull_ff_only(root: String) -> Result<GitSnapshot, String> {
    let root = canonical_root(&root)?;
    // Solo permite avance rápido: evita crear merges inesperados en el workspace.
    run_git(&root, &["pull", "--ff-only"])?;
    snapshot(&root)
}

#[tauri::command]
pub fn git_push(root: String) -> Result<GitSnapshot, String> {
    let root = canonical_root(&root)?;
    run_git(&root, &["push"])?;
    snapshot(&root)
}
