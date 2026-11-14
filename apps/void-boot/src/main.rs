use std::{
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};
use void_boot::{devices::list_removable_devices, profiles::ProfileCatalog, scan_directory, IsoEntry};

fn print_entries(entries: &[IsoEntry], json: bool) -> Result<(), Box<dyn std::error::Error>> {
    if json {
        println!("{}", serde_json::to_string_pretty(entries)?);
    } else if entries.is_empty() {
        println!("No ISO files found.");
    } else {
        for entry in entries {
            println!(
                "{} — {} bytes — {} — {}",
                entry.name, entry.size_bytes, entry.status, entry.sha256
            );
        }
        println!(
            "\n{} ISO file(s) inspected. No drives were modified.",
            entries.len()
        );
    }
    Ok(())
}

fn render_menu(
    profile_file: &Path,
    output_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let catalog = ProfileCatalog::load(profile_file)?;
    fs::create_dir_all(output_dir)?;
    let theme_dir = output_dir.join("themes").join("void-boot");
    fs::create_dir_all(&theme_dir)?;

    fs::write(output_dir.join("grub.cfg"), catalog.render_grub_config())?;
    let theme_source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("grub")
        .join("theme.txt");
    fs::copy(theme_source, theme_dir.join("theme.txt"))?;

    println!(
        "Generated graphical GRUB menu for {} profile(s) at {}",
        catalog.profiles.len(),
        output_dir.display()
    );
    println!("No bootloader was installed and no storage device was modified.");
    Ok(())
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("scan") => {
            let directory = args.next().ok_or("missing directory argument")?;
            let json = args.any(|arg| arg == "--json");
            let entries = scan_directory(Path::new(&directory))?;
            print_entries(&entries, json)
        }
        Some("list-devices") => {
            let json = args.any(|arg| arg == "--json");
            let devices = list_removable_devices()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&devices)?);
            } else if devices.is_empty() {
                println!("No removable block devices detected.");
            } else {
                for device in &devices {
                    println!(
                        "{} — {} — {} bytes — {} — {}",
                        device.device_path,
                        if device.model.is_empty() { "Unknown model" } else { &device.model },
                        device.size_bytes,
                        if device.read_only { "read-only" } else { "writable" },
                        if device.mounted { "mounted" } else { "not mounted" }
                    );
                }
            }
            println!("Read-only inventory: no device was opened for writing or modified.");
            Ok(())
        }
        Some("validate-profiles") => {
            let profile_file = args.next().ok_or("missing profile catalog path")?;
            let catalog = ProfileCatalog::load(Path::new(&profile_file))?;
            println!("Profile catalog is valid: {} profile(s).", catalog.profiles.len());
            for profile in &catalog.profiles {
                println!("{} — {} ({})", profile.id, profile.name, profile.iso);
            }
            Ok(())
        }
        Some("render-menu") => {
            let profile_file = args.next().ok_or("missing profile catalog path")?;
            let output_dir = args.next().ok_or("missing output directory")?;
            render_menu(Path::new(&profile_file), Path::new(&output_dir))
        }
        _ => {
            eprintln!(
                "Void Boot — multiboot tools\n\nUsage:\n  \
                 void-boot scan <directory> [--json]\n  \
                 void-boot render-menu <profiles.toml> <output-directory>"
            );
            Err("expected 'scan' or 'render-menu' command".into())
        }
    }
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
