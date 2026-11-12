use std::io::{self, Write};

pub fn run() -> Result<(), String> {
    loop {
        print!("\x1b[2J\x1b[H");
        println!("╔══════════════════════════════════════╗");
        println!("║          VOID MEMINGO TUI            ║");
        println!("║       OS DEVELOPMENT CONSOLE         ║");
        println!("╚══════════════════════════════════════╝");
        println!();
        println!("  [1] Initialize a new OS project");
        println!("  [2] Check current project");
        println!("  [3] Show project configuration");
        println!("  [4] Run a .void language script");
        println!("  [5] Install/update Void Memingo");
        println!("  [6] Show version");
        println!("  [0] Exit");
        print!("\n  void> ");
        io::stdout().flush().map_err(|e| e.to_string())?;
        let mut choice = String::new();
        io::stdin().read_line(&mut choice).map_err(|e| e.to_string())?;

        let result = match choice.trim() {
            "1" => {
                let path = ask("Project directory")?;
                crate::initialize(std::path::Path::new(&path))
            }
            "2" => crate::check_project(std::path::Path::new(".")),
            "3" => crate::show_config(std::path::Path::new("void.toml")),
            "4" => {
                let path = ask("Script path")?;
                crate::language::run_file(std::path::Path::new(&path))
            }
            "5" => crate::installer::run(),
            "6" => { println!("Void Memingo {}", env!("CARGO_PKG_VERSION")); Ok(()) }
            "0" => return Ok(()),
            _ => Err("Unknown menu option.".into()),
        };

        if let Err(error) = result { println!("\nError: {}", error); }
        println!("\nPress Enter to return to the menu...");
        let mut pause = String::new();
        io::stdin().read_line(&mut pause).map_err(|e| e.to_string())?;
    }
}

fn ask(label: &str) -> Result<String, String> {
    print!("{}: ", label);
    io::stdout().flush().map_err(|e| e.to_string())?;
    let mut value = String::new();
    io::stdin().read_line(&mut value).map_err(|e| e.to_string())?;
    Ok(value.trim().to_string())
}
