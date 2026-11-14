use serde::Serialize;
use std::{fs, io, path::Path};

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct RemovableDevice {
    pub name: String,
    pub device_path: String,
    pub model: String,
    pub vendor: String,
    pub size_bytes: u64,
    pub read_only: bool,
    pub mounted: bool,
}

#[cfg(target_os = "linux")]
pub fn list_removable_devices() -> io::Result<Vec<RemovableDevice>> {
    let block_root = Path::new("/sys/block");
    let mut devices = Vec::new();

    for entry in fs::read_dir(block_root)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();

        // Ignore virtual block devices; this inventory is for removable media only.
        if name.starts_with("loop") || name.starts_with("ram") || name.starts_with("zram") {
            continue;
        }

        let sys_path = entry.path();
        let removable = read_trimmed(&sys_path.join("removable"))?;
        if removable != "1" {
            continue;
        }

        let sectors = read_trimmed(&sys_path.join("size"))?
            .parse::<u64>()
            .unwrap_or(0);
        let device_path = format!("/dev/{name}");
        let read_only = read_trimmed(&sys_path.join("ro"))
            .map(|value| value == "1")
            .unwrap_or(true);

        devices.push(RemovableDevice {
            name,
            device_path: device_path.clone(),
            model: read_trimmed(&sys_path.join("device/model")).unwrap_or_default(),
            vendor: read_trimmed(&sys_path.join("device/vendor")).unwrap_or_default(),
            size_bytes: sectors.saturating_mul(512),
            read_only,
            mounted: is_device_mounted(&device_path)?,
        });
    }

    devices.sort_by(|left, right| left.device_path.cmp(&right.device_path));
    Ok(devices)
}

#[cfg(not(target_os = "linux"))]
pub fn list_removable_devices() -> io::Result<Vec<RemovableDevice>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "read-only removable-device discovery is currently supported on Linux only",
    ))
}

#[cfg(target_os = "linux")]
fn read_trimmed(path: &Path) -> io::Result<String> {
    Ok(fs::read_to_string(path)?.trim().to_string())
}

#[cfg(target_os = "linux")]
fn is_device_mounted(device_path: &str) -> io::Result<bool> {
    let mountinfo = fs::read_to_string("/proc/self/mountinfo")?;
    Ok(mountinfo.lines().any(|line| {
        line.split(" - ")
            .nth(1)
            .and_then(|right| right.split_whitespace().nth(1))
            .is_some_and(|source| {
                source == device_path
                    || source.strip_prefix(device_path).is_some_and(|suffix| {
                        suffix.starts_with('p')
                            || suffix.chars().next().is_some_and(|c| c.is_ascii_digit())
                    })
            })
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_model_fields_are_serializable() {
        let device = RemovableDevice {
            name: "sdb".into(),
            device_path: "/dev/sdb".into(),
            model: "Virtual USB".into(),
            vendor: "Atdev".into(),
            size_bytes: 1024,
            read_only: false,
            mounted: false,
        };
        let json = serde_json::to_string(&device).expect("serialize removable device");
        assert!(json.contains("/dev/sdb"));
        assert!(json.contains("Virtual USB"));
    }
}
