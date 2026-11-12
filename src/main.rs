mod config;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const VERSION: &str = "0.1.0";

fn main() {
    let args: Vec<String> = env::args().collect();

    let result = match args.get(1).map(String::as_str) {
        Some("version") | Some("--version") | Some("-V") => {
            println!("Void Memingo {}", VERSION);
            Ok(())
        }
        Some("init") => {
            let path = argument_path(&args);
            initialize(&path)
        }
        Some("check") => {
            let path = argument_path(&args);
            check_project(&path)
        }
        Some("config") => {
            let path = argument_path(&args).join("void.toml");
            show_config(&path)
        }
        Some("help") | Some("--help") | Some("-h") | None => {
            show_help();
            Ok(())
        }
        Some(command) => {
            show_help();
            Err(format!("Unknown command: {}", command))
        }
    };

    if let Err(error) = result {
        eprintln!("Void error: {}", error);
        std::process::exit(1);
    }
}

fn argument_path(args: &[String]) -> PathBuf {
    args.get(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn show_help() {
    println!("Void Memingo {}", VERSION);
    println!();
    println!("Usage:");
    println!("  void version       Show version");
    println!("  void init [path]   Initialize a project");
    println!("  void check [path]  Validate a project");
    println!("  void config [path] Display project configuration");
    println!("  void help          Show this help");
}

fn initialize(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path)
        .map_err(|e| format!("Cannot create project directory: {}", e))?;

    let config_path = path.join("void.toml");
    if config_path.exists() {
        return Err("void.toml already exists. Initialization cancelled.".into());
    }

    let project_name = path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("void-project")
        .replace('\\', "-")
        .replace('"', "\\\"");

    let config_content = format!(
        "[project]\nname = \"{}\"\nversion = \"0.1.0\"\n\n[build]\ntarget = \"x86_64-unknown-none\"\n",
        project_name
    );

    fs::write(&config_path, config_content)
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
    println!("Configuration: {}", config_path.display());
    Ok(())
}

fn show_config(path: &Path) -> Result<(), String> {
    let settings = config::load(path)?;

    println!("Void Memingo project configuration");
    println!("  Name:    {}", settings.project.name);
    println!("  Version: {}", settings.project.version);
    println!("  Target:  {}", settings.build.target);

    Ok(())
}

fn check_project(path: &Path) -> Result<(), String> {
    let settings = config::load(&path.join("void.toml"))?;

    let required = [
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
    println!("  Project: {}", settings.project.name);
    println!("  Version: {}", settings.project.version);
    println!("  Target:  {}", settings.build.target);
    println!("All required directories are present.");

    Ok(())
}
