mod config;
mod language;
mod tui;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args: Vec<String> = env::args().collect();

    let result = match args.get(1).map(String::as_str) {
        Some("version") | Some("--version") | Some("-V") => {
            println!("Void Memingo {}", VERSION);
            Ok(())
        }
        Some("init") => initialize(&argument_path(&args)),
        Some("check") => check_project(&argument_path(&args)),
        Some("config") => show_config(&argument_path(&args).join("void.toml")),
        Some("tui") => tui::run(),
        Some("run") => match args.get(2) {
            Some(file) => language::run_file(Path::new(file)),
            None => Err("Usage: void run <file.void>".into()),
        },
        Some("install") => installer::run(),
        Some("help") | Some("--help") | Some("-h") | None => {
            show_help();
            Ok(())
        }
        Some(command) => Err(format!("Unknown command: {}", command)),
    };

    if let Err(error) = result {
        eprintln!("Void error: {}", error);
        std::process::exit(1);
    }
}

mod installer {
    use reqwest::blocking::Client;
    use sha2::{Digest, Sha256};
    use std::fs;
    use std::io::{self, Write};
    use std::path::PathBuf;

    const REPO: &str = "Reaccionador2Jeje/Void-Memingo";

    pub fn run() -> Result<(), String> {
        println!("Void Memingo Installer");
        println!("This installer downloads the official Linux x86_64 release.");
        if !cfg!(target_os = "linux") || !cfg!(target_arch = "x86_64") {
            return Err("The prebuilt installer currently supports Linux x86_64 only.".into());
        }

        print!("Install to [1] ~/.local/bin (recommended) or [2] /usr/local/bin? ");
        io::stdout().flush().map_err(|e| e.to_string())?;
        let mut choice = String::new();
        io::stdin().read_line(&mut choice).map_err(|e| e.to_string())?;
        let destination = if choice.trim() == "2" {
            PathBuf::from("/usr/local/bin")
        } else {
            PathBuf::from(std::env::var("HOME").map_err(|e| e.to_string())?).join(".local/bin")
        };

        let base = format!("https://github.com/{}/releases/latest/download", REPO);
        let client = Client::builder().user_agent(concat!("void-memingo/", env!("CARGO_PKG_VERSION")))
            .build().map_err(|e| format!("Cannot initialize HTTPS client: {}", e))?;

        println!("Checking the latest GitHub release...");
        let binary_url = format!("{}/void-linux-x86_64", base);
        let checksum_url = format!("{}/void-linux-x86_64.sha256", base);
        let binary = client.get(&binary_url).send()
            .map_err(|e| format!("Download failed: {}", e))?;
        if !binary.status().is_success() {
            return Err(format!("GitHub returned HTTP {}. The release asset may not exist yet. Publish a GitHub Release first.", binary.status()));
        }
        let bytes = binary.bytes().map_err(|e| e.to_string())?;

        let checksum_response = client.get(&checksum_url).send().map_err(|e| e.to_string())?;
        if !checksum_response.status().is_success() {
            return Err(format!("Checksum download failed: HTTP {}", checksum_response.status()));
        }
        let checksum_text = checksum_response.text().map_err(|e| e.to_string())?;
        let expected = checksum_text.split_whitespace().next()
            .ok_or("The checksum file is empty.")?;

        let actual = format!("{:x}", Sha256::digest(&bytes));
        if actual != expected {
            return Err("SHA-256 verification failed. The downloaded file was not installed.".into());
        }

        fs::create_dir_all(&destination).map_err(|e| format!("Cannot create {}: {}", destination.display(), e))?;
        let target = destination.join("void");
        let temporary = destination.join("void.download");
        fs::write(&temporary, &bytes).map_err(|e| e.to_string())?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&temporary, fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
        }
        fs::rename(&temporary, &target).map_err(|e| format!("Cannot install binary: {}", e))?;
        println!("Installed successfully: {}", target.display());
        println!("Run: {}/void tui", destination.display());
        Ok(())
    }
}

fn argument_path(args: &[String]) -> PathBuf {
    args.get(2).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."))
}

fn show_help() {
    println!("Void Memingo {}", VERSION);
    println!("Usage:");
    println!("  void tui             Open the interactive terminal interface");
    println!("  void install         Install/update Void from the official release");
    println!("  void run <file.void> Run a Void language script");
    println!("  void init [path]     Initialize an OS project");
    println!("  void check [path]    Validate an OS project");
    println!("  void config [path]   Display project configuration");
    println!("  void version         Show version");
    println!("  void help            Show this help");
}

fn initialize(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| format!("Cannot create project directory: {}", e))?;
    let config_path = path.join("void.toml");
    if config_path.exists() {
        return Err("void.toml already exists. Initialization cancelled.".into());
    }
    let project_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("void-project")
        .replace('\\', "-").replace('"', "\\\"");
    let config = format!("[project]\nname = \"{}\"\nversion = \"0.1.0\"\n\n[build]\ntarget = \"x86_64-unknown-none\"\n", project_name);
    fs::write(&config_path, config).map_err(|e| e.to_string())?;
    for dir in ["compiler", "os/kernel", "os/boot", "distro/packages", "distro/config", "tools", "runtime", "examples", "tests", "docs"] {
        fs::create_dir_all(path.join(dir)).map_err(|e| format!("Cannot create {}: {}", dir, e))?;
    }
    println!("Void project initialized: {}", path.display());
    Ok(())
}

fn show_config(path: &Path) -> Result<(), String> {
    let settings = config::load(path)?;
    println!("Project: {}\nVersion: {}\nTarget: {}", settings.project.name, settings.project.version, settings.build.target);
    Ok(())
}

fn check_project(path: &Path) -> Result<(), String> {
    let settings = config::load(&path.join("void.toml"))?;
    let required = ["compiler", "os/kernel", "os/boot", "distro/packages", "distro/config", "tools", "runtime", "examples", "tests", "docs"];
    let missing: Vec<_> = required.iter().filter(|d| !path.join(d).is_dir()).collect();
    if !missing.is_empty() {
        for dir in missing { println!("Missing: {}", dir); }
        return Err("Project structure is incomplete.".into());
    }
    println!("Project check passed: {} ({})", settings.project.name, settings.project.version);
    Ok(())
}
