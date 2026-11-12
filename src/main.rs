use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const VERSION: &str = "0.1.0";

fn main() {
    let args: Vec<String> = env::args().collect();

    match args.get(1).map(String::as_str) {
        Some("version") | Some("--version") | Some("-V") => {
            println!("Void Memingo {}", VERSION);
        }
        Some("init") => {
            let path = args.get(2)
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));

            if let Err(error) = initialize(&path) {
                eprintln!("Void error: {}", error);
                std::process::exit(1);
            }
        }
        Some("check") => {
            let path = args.get(2)
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));

            if let Err(error) = check_project(&path) {
                eprintln!("Void error: {}", error);
                std::process::exit(1);
            }
        }
        Some("help") | Some("--help") | Some("-h") | None => show_help(),
        Some(command) => {
            eprintln!("Unknown command: {}", command);
            show_help();
            std::process::exit(1);
        }
    }
}

fn show_help() {
    println!("Void Memingo {}", VERSION);
    println!();
    println!("Usage:");
    println!("  void version       Show version");
    println!("  void init [path]   Initialize a project");
    println!("  void check [path]  Check project structure");
    println!("  void help          Show this help");
}

fn initialize(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path)
        .map_err(|e| format!("Cannot create project directory: {}", e))?;

    let config = path.join("void.toml");
    if config.exists() {
        return Err("void.toml already exists. Initialization cancelled.".into());
    }

    let project_name = path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("void-project");

    let config_content = format!(
        "[project]\nname = \"{}\"\nversion = \"0.1.0\"\n\n[build]\ntarget = \"x86_64-unknown-none\"\n",
        project_name
    );

    fs::write(&config, config_content)
        .map_err(|e| format!("Cannot create void.toml: {}", e))?;

    let directories = [
        "compiler",
        "os/kernel",
        "os/boot",
        "distro/packages",
        "distro/config",
        "tools",
        "runtime",
        "examples",
        "tests",
        "docs",
    ];

    for directory in directories {
        fs::create_dir_all(path.join(directory))
            .map_err(|e| format!("Cannot create {}: {}", directory, e))?;
    }

    println!("Void Memingo project initialized!");
    println!("Project: {}", project_name);
    println!("Configuration: void.toml");
    Ok(())
}

fn check_project(path: &Path) -> Result<(), String> {
    if !path.join("void.toml").is_file() {
        return Err("No void.toml found. Run 'void init' first.".into());
    }

    let required = [
        "compiler", "os/kernel", "os/boot", "distro/packages",
        "distro/config", "tools", "runtime", "examples", "tests", "docs",
    ];

    let missing: Vec<&str> = required.iter()
        .copied()
        .filter(|directory| !path.join(directory).is_dir())
        .collect();

    if !missing.is_empty() {
        println!("Project structure is incomplete:");
        for directory in missing {
            println!("  Missing: {}", directory);
        }
        return Err("Project check failed.".into());
    }

    println!("Void Memingo project check passed!");
    println!("Configuration found.");
    println!("All required directories are present.");
    Ok(())
}
