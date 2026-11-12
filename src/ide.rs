use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn run() -> Result<(), String> {
    loop {
        clear();
        println!("╔════════════════════════════════════════════════════════════╗");
        println!("║                    VOID MEMINGO IDE                         ║");
        println!("║              OPERATING SYSTEM DEVELOPMENT                   ║");
        println!("╚════════════════════════════════════════════════════════════╝");
        println!();
        println!("  [1] Open project workspace");
        println!("  [2] Create new OS project");
        println!("  [3] Check project in current directory");
        println!("  [4] Run a .void script");
        println!("  [5] Toolchain diagnostics");
        println!("  [6] Install/update Void Memingo");
        println!("  [7] About / version");
        println!("  [0] Exit");
        match prompt("\n  void> ")?.as_str() {
            "1" => {
                let path = prompt("Project directory [.] > ")?;
                let root = if path.trim().is_empty() { env::current_dir().map_err(err)? } else { PathBuf::from(path.trim()) };
                if let Err(e) = workspace(&root) { report(&e); }
                pause()?;
            }
            "2" => {
                let path = prompt("New project directory > ")?;
                if path.trim().is_empty() { report("A project directory is required."); }
                else if let Err(e) = crate::initialize(Path::new(path.trim())) { report(&e); }
                else { println!("\nProject created. Open it with option 1."); }
                pause()?;
            }
            "3" => {
                if let Err(e) = crate::check_project(Path::new(".")) { report(&e); }
                pause()?;
            }
            "4" => {
                let path = prompt("Script path > ")?;
                if let Err(e) = crate::language::run_file(Path::new(path.trim())) { report(&e); }
                pause()?;
            }
            "5" => { diagnostics(); pause()?; }
            "6" => {
                if let Err(e) = crate::installer::run() { report(&e); }
                pause()?;
            }
            "7" => {
                println!("\nVoid Memingo {}", env!("CARGO_PKG_VERSION"));
                println!("An OS development toolkit and scripting environment.");
                pause()?;
            }
            "0" => return Ok(()),
            _ => { report("Unknown option."); pause()?; }
        }
    }
}

fn workspace(root: &Path) -> Result<(), String> {
    let root = root.canonicalize().map_err(|e| format!("Cannot open project directory: {e}"))?;
    if !root.is_dir() { return Err("The selected path is not a directory.".into()); }
    loop {
        clear();
        let name = root.file_name().and_then(|v| v.to_str()).unwrap_or("workspace");
        println!("╔════════════════════════════════════════════════════════════╗");
        println!("║  VOID MEMINGO IDE  /  WORKSPACE                             ║");
        println!("╚════════════════════════════════════════════════════════════╝");
        println!("  Project: {}\n", name);
        println!("  [1] Project Explorer");
        println!("  [2] Open file in editor ($VISUAL / $EDITOR)");
        println!("  [3] Run a .void script");
        println!("  [4] Validate project");
        println!("  [5] Inspect build toolchain");
        println!("  [6] Show project configuration");
        println!("  [0] Back to main menu");
        match prompt("\n  workspace> ")?.as_str() {
            "1" => { explorer(&root, &root, 0)?; pause()?; }
            "2" => { edit_file(&root)?; pause()?; }
            "3" => {
                let rel = prompt("Script path (relative to project) > ")?;
                match safe_existing_file(&root, &rel) {
                    Ok(path) => if let Err(e) = crate::language::run_file(&path) { report(&e); },
                    Err(e) => report(&e),
                }
                pause()?;
            }
            "4" => { if let Err(e) = crate::check_project(&root) { report(&e); } pause()?; }
            "5" => { diagnostics(); pause()?; }
            "6" => {
                if let Err(e) = crate::show_config(&root.join("void.toml")) { report(&e); }
                pause()?;
            }
            "0" => return Ok(()),
            _ => { report("Unknown option."); pause()?; }
        }
    }
}

fn explorer(root: &Path, dir: &Path, depth: usize) -> Result<(), String> {
    if depth == 0 {
        println!("\nPROJECT EXPLORER\n");
        println!("{}", root.display());
    }
    if depth > 4 { return Ok(()); }
    let mut entries = fs::read_dir(dir).map_err(|e| format!("Cannot read {}: {e}", dir.display()))?
        .filter_map(Result::ok).collect::<Vec<_>>();
    entries.sort_by_key(|e| ( !e.path().is_dir(), e.file_name().to_string_lossy().to_lowercase()));
    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name == "target" { continue; }
        let path = entry.path();
        let kind = if path.is_dir() { "📁" } else { "📄" };
        println!("{}{} {}", "  ".repeat(depth + 1), kind, name);
        if path.is_dir() { explorer(root, &path, depth + 1)?; }
    }
    Ok(())
}

fn edit_file(root: &Path) -> Result<(), String> {
    let rel = prompt("File path (relative to project) > ")?;
    let path = safe_existing_file(root, &rel)?;
    let editor = env::var("VISUAL").or_else(|_| env::var("EDITOR"))
        .unwrap_or_else(|_| "nano".to_string());
    let mut parts = editor.split_whitespace();
    let command = parts.next().ok_or("Editor command is empty.")?;
    let status = Command::new(command).args(parts).arg(&path).status()
        .map_err(|e| format!("Cannot launch editor '{command}': {e}. Set EDITOR to an installed editor."))?;
    if !status.success() { return Err(format!("Editor exited with status {status}")); }
    Ok(())
}

fn safe_existing_file(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let candidate = root.join(relative);
    let path = candidate.canonicalize().map_err(|e| format!("Cannot open '{}': {e}", relative))?;
    if !path.starts_with(root) { return Err("The requested file is outside this project.".into()); }
    if !path.is_file() { return Err("The selected path is not a file.".into()); }
    Ok(path)
}

fn diagnostics() {
    println!("\nTOOLCHAIN DIAGNOSTICS\n");
    for tool in ["rustc", "cargo", "nasm", "ld", "qemu-system-x86_64", "make", "git"] {
        let result = Command::new(tool).arg("--version").output();
        match result {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let line = stdout.lines().next().or_else(|| stderr.lines().next()).unwrap_or("Available");
                println!("  [OK]   {tool}: {line}");
            }
            Ok(_) => println!("  [WARN] {tool}: command returned an error"),
            Err(_) => println!("  [MISS] {tool}: not found in PATH"),
        }
    }
    println!("\nMissing tools are optional until a project needs them.");
}

fn prompt(label: &str) -> Result<String, String> {
    print!("{label}");
    io::stdout().flush().map_err(err)?;
    let mut value = String::new();
    let bytes = io::stdin().read_line(&mut value).map_err(err)?;
    if bytes == 0 { return Err("Input closed.".into()); }
    Ok(value.trim().to_string())
}

fn pause() -> Result<(), String> {
    let _ = prompt("\nPress Enter to continue...")?;
    Ok(())
}

fn report(message: &str) { println!("\nError: {message}"); }
fn clear() { print!("\x1b[2J\x1b[H"); let _ = io::stdout().flush(); }
fn err(error: io::Error) -> String { error.to_string() }
