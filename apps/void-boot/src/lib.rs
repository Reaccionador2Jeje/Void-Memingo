use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{self, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

const SECTOR_SIZE: u64 = 2048;
const FIRST_VOLUME_DESCRIPTOR_SECTOR: u64 = 16;
const MAX_VOLUME_DESCRIPTOR_SECTORS: u64 = 64;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct IsoEntry {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub iso9660: bool,
    pub el_torito_descriptor: bool,
    pub status: &'static str,
}

fn read_volume_descriptor(file: &mut File, sector: u64) -> io::Result<Option<[u8; 2048]>> {
    let mut descriptor = [0_u8; SECTOR_SIZE as usize];
    file.seek(SeekFrom::Start(sector * SECTOR_SIZE))?;
    match file.read_exact(&mut descriptor) {
        Ok(()) => Ok(Some(descriptor)),
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => Ok(None),
        Err(error) => Err(error),
    }
}

fn is_valid_volume_descriptor(descriptor: &[u8; 2048]) -> bool {
    &descriptor[1..6] == b"CD001" && descriptor[6] == 1
}

fn inspect_volume_descriptors(file: &mut File) -> io::Result<(bool, bool)> {
    let mut has_primary = false;
    let mut has_el_torito = false;

    for sector in FIRST_VOLUME_DESCRIPTOR_SECTOR..MAX_VOLUME_DESCRIPTOR_SECTORS {
        let Some(descriptor) = read_volume_descriptor(file, sector)? else {
            break;
        };
        if !is_valid_volume_descriptor(&descriptor) {
            break;
        }

        match descriptor[0] {
            0 if &descriptor[7..30] == b"EL TORITO SPECIFICATION" => {
                has_el_torito = true;
            }
            1 => has_primary = true,
            255 => break,
            _ => {}
        }
    }

    Ok((has_primary, has_el_torito))
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

pub fn inspect_iso(path: &Path) -> io::Result<IsoEntry> {
    let mut file = File::open(path)?;
    let metadata = file.metadata()?;
    let (iso9660, el_torito_descriptor) = inspect_volume_descriptors(&mut file)?;
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

pub fn scan_directory(directory: &Path) -> io::Result<Vec<IsoEntry>> {
    if !directory.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "the supplied path is not a directory",
        ));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::Write,
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    fn unique_temp_path(label: &str) -> PathBuf {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after UNIX epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "void-boot-{}-{stamp}-{id}-{label}",
            std::process::id()
        ))
    }

    fn descriptor(kind: u8, identifier: &[u8]) -> [u8; 2048] {
        let mut bytes = [0_u8; 2048];
        bytes[0] = kind;
        bytes[1..6].copy_from_slice(b"CD001");
        bytes[6] = 1;
        bytes[7..7 + identifier.len()].copy_from_slice(identifier);
        bytes
    }

    #[test]
    fn scans_iso9660_and_el_torito_descriptors_from_file() {
        let path = unique_temp_path("bootable.iso");
        let mut bytes = vec![0_u8; 18 * 2048];
        bytes[16 * 2048..17 * 2048].copy_from_slice(&descriptor(1, b""));
        bytes[17 * 2048..18 * 2048]
            .copy_from_slice(&descriptor(0, b"EL TORITO SPECIFICATION"));

        {
            let mut file = File::create(&path).expect("create fixture ISO");
            file.write_all(&bytes).expect("write fixture ISO");
        }

        let entry = inspect_iso(&path).expect("inspect fixture ISO");
        assert!(entry.iso9660);
        assert!(entry.el_torito_descriptor);
        assert_eq!(entry.status, "iso9660_with_el_torito_descriptor");
        assert_eq!(entry.size_bytes, bytes.len() as u64);

        fs::remove_file(path).expect("remove fixture ISO");
    }

    #[test]
    fn computes_sha256_for_iso_contents() {
        let path = unique_temp_path("hash.iso");
        fs::write(&path, b"abc").expect("write hash fixture");
        let mut file = File::open(&path).expect("open hash fixture");
        assert_eq!(
            sha256_file(&mut file).expect("calculate SHA-256"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        fs::remove_file(path).expect("remove hash fixture");
    }

    #[test]
    fn recognizes_primary_volume_descriptor() {
        let pvd = descriptor(1, b"");
        assert!(is_valid_volume_descriptor(&pvd));
        assert_eq!(pvd[0], 1);
    }

    #[test]
    fn recognizes_el_torito_boot_record() {
        let boot = descriptor(0, b"EL TORITO SPECIFICATION");
        assert!(is_valid_volume_descriptor(&boot));
        assert_eq!(&boot[7..30], b"EL TORITO SPECIFICATION");
    }

    #[test]
    fn rejects_invalid_descriptor_identifier() {
        let mut invalid = descriptor(1, b"");
        invalid[1] = b'X';
        assert!(!is_valid_volume_descriptor(&invalid));
    }

    #[test]
    fn rejects_wrong_descriptor_version() {
        let mut invalid = descriptor(1, b"");
        invalid[6] = 2;
        assert!(!is_valid_volume_descriptor(&invalid));
    }
}
