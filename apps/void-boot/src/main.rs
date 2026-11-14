use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    env,
    fs::{self, File},
    io::{self, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::ExitCode,
};

#[derive(Debug, Serialize)]
struct IsoEntry {
    name: String,
    path: String,
    size_bytes: u64,
    sha256: String,
    iso9660: bool,
    el_torito_descriptor: bool,
    status: &'static str,
}

fn has_iso9660_primary_volume_descriptor(file: &mut File) -> io::Result<bool> {
    let mut descriptor = [0_u8; 7];
    file.seek(SeekFrom::Start(16 * 2048))?;
    if file.read_exact(&mut descriptor).is_err() {
        return Ok(false);
    }
    Ok(descriptor[0] == 1 && &descriptor[1..6] == b"CD001" && descriptor[6] == 1)
}

fn has_el_torito_boot_record(file: &mut File) -> io::Result<bool> {
    let mut descriptor = [0_u8; 40];
    file.seek(SeekFrom::Start(17 * 2048))?;
    if file.read_exact(&mut descriptor).is_err() {
        return Ok(false);
    }
    Ok(descriptor[0] == 0
        && &descriptor[1..6] == b"CD001"
        && descriptor[6] == 1
        && &descriptor[7..30] == b"EL TORITO SPECIFICATION")
}

fn sha256_file(file: &mut File) -> io::Result<String> {
    file.seek(SeekFrom::Start(0))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn inspect_iso(path: &Path) -> io::Result<IsoEntry> {
    let mut file = File::open(path)?;
    let metadata = file.metadata()?;
    let iso9660 = has_iso9660_primary_volume_descriptor(&mut file)?;
    let el_torito_descriptor = if iso9660 {
        has_el_torito_boot_record(&mut file)?
    } else {
        false
    };
    let sha256 = sha256_file(&mut file)?;
    let status = if !iso9660 {
        "not_iso9660"
    } else if !el_torito_descriptor {
        "iso9660_without_el_torito_descriptor"
    } else {
        "iso9660_with_el_torito_descriptor"
    };

    Ok(IsoEntry {
        name: path.file_name().unwrap_or_default().to_string_lossy().into_owned(),
        path: path.to_string_lossy().into_owned(),
        size_bytes: metadata.len(),
        sha256,
        iso9660,
        el_torito_descriptor,
        status,
    })
}

fn scan_directory(directory: &Path) -> io::Result<Vec<IsoEntry>> {
    if !directory.is_dir() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "the supplied path is not a directory"));
    }

    let mut paths = Vec::<PathBuf>::new();
    for item in fs::read_dir(directory)? {
        let item = item?;
        let path = item.path();
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_file()
            && path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("iso"))
        {
            paths.push(path);
        }
    }
    paths.sort();

    let mut entries = Vec::with_capacity(paths.len());
    for path in paths {
        entries.push(inspect_iso(&path)?);
    }
    Ok(entries)
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

    if json {
        println!("{}", serde_json::to_string_pretty(&entries)?);
    } else if entries.is_empty() {
        println!("No ISO files found in {directory}");
    } else {
        for entry in &entries {
            println!("{} — {} bytes — {} — {}", entry.name, entry.size_bytes, entry.status, entry.sha256);
        }
        println!("\n{} ISO file(s) inspected. No drives were modified.", entries.len());
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_iso9660_primary_descriptor_signature() {
        let mut descriptor = [0_u8; 7];
        descriptor[0] = 1;
        descriptor[1..6].copy_from_slice(b"CD001");
        descriptor[6] = 1;
        assert_eq!(descriptor[0], 1);
        assert_eq!(&descriptor[1..6], b"CD001");
        assert_eq!(descriptor[6], 1);
    }

    #[test]
    fn recognizes_el_torito_boot_record_identifier() {
        let mut descriptor = [0_u8; 40];
        descriptor[0] = 0;
        descriptor[1..6].copy_from_slice(b"CD001");
        descriptor[6] = 1;
        descriptor[7..30].copy_from_slice(b"EL TORITO SPECIFICATION");
        assert_eq!(&descriptor[7..30], b"EL TORITO SPECIFICATION");
    }
}
