use std::{env, path::Path, process::ExitCode};
use void_boot::{scan_directory, IsoEntry};

fn print_entries(entries: &[IsoEntry], json: bool) -> Result<(), Box<dyn std::error::Error>> {
    if json {
        println!("{}", serde_json::to_string_pretty(entries)?);
    } else if entries.is_empty() {
        println!("No ISO files found.");
    } else {
        for entry in entries {
            println!("{} — {} bytes — {} — {}", entry.name, entry.size_bytes, entry.status, entry.sha256);
        }
        println!("\n{} ISO file(s) inspected. No drives were modified.", entries.len());
    }
    Ok(())
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_default();
    if command != "scan" {
        eprintln!("Void Boot — safe ISO inventory\n\nUsage: void-boot scan <directory> [--json]");
        return Err("expected the 'scan' command".into());
    }
    let directory = args.next().ok_or("missing directory argument")?;
    let json = args.any(|arg| arg == "--json");
    let entries = scan_directory(Path::new(&directory))?;
    print_entries(&entries, json)
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("void-boot: {error}");
            ExitCode::FAILURE
        }
    }
}
