use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, io, path::Path};

#[derive(Debug, Deserialize, Serialize)]
pub struct ProfileCatalog {
    pub profiles: Vec<BootProfile>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct BootProfile {
    pub id: String,
    pub name: String,
    pub iso: String,
    pub kernel: String,
    pub initrd: String,
    #[serde(default)]
    pub linux_args: Vec<String>,
    #[serde(default)]
    pub findiso: bool,
    /// Kernel parameter key that receives the ISO path, e.g. iso-scan/filename.
    #[serde(default)]
    pub iso_path_argument: Option<String>,
    /// Load a distribution-provided GRUB loopback recipe from inside the ISO.
    #[serde(default)]
    pub loopback_config: Option<String>,
    /// Use GRUB source instead of configfile for the embedded recipe.
    #[serde(default)]
    pub loopback_config_source: bool,
}

fn safe_relative_path(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('/')
        && !value.starts_with('\\')
        && value.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && part
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || "._-".contains(character))
        })
}

fn safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '-' || character == '_'
        })
}

fn safe_label(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || " ._()-".contains(character))
}

fn safe_argument(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "._/,:=+-".contains(character))
}

impl ProfileCatalog {
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = fs::read_to_string(path)?;
        let catalog: Self = toml::from_str(&contents)?;
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn validate(&self) -> io::Result<()> {
        if self.profiles.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "profile catalog is empty",
            ));
        }
        if self.profiles.len() > 128 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "profile catalog exceeds the 128-profile limit",
            ));
        }

        let mut profile_ids = HashSet::new();
        for profile in &self.profiles {
            if !safe_id(&profile.id) || !profile_ids.insert(&profile.id) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "unsafe profile id",
                ));
            }
            if !safe_label(&profile.name) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "unsafe profile name",
                ));
            }
            if !safe_relative_path(&profile.iso)
                || !profile.iso.to_ascii_lowercase().ends_with(".iso")
                || !safe_relative_path(&profile.kernel)
                || !safe_relative_path(&profile.initrd)
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "ISO, kernel and initrd paths must be safe relative paths; ISO paths must end in .iso",
                ));
            }
            if profile.iso_path_argument.as_ref().is_some_and(|value| {
                value.is_empty() || value.len() > 96 || !value.chars().all(|character| character.is_ascii_alphanumeric() || "._/-".contains(character))
            }) {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "unsafe ISO path kernel parameter"));
            }
            if profile.loopback_config.as_ref().is_some_and(|value| !safe_relative_path(value)) {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "unsafe embedded loopback config path"));
            }
            if profile.loopback_config_source && profile.loopback_config.is_none() {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "loopback_config_source requires loopback_config"));
            }
            if profile.findiso && profile.iso_path_argument.is_some() {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "choose either findiso or iso_path_argument, not both"));
            }
            if profile.linux_args.len() > 64
                || profile
                    .linux_args
                    .iter()
                    .any(|arg| arg.len() > 256 || !safe_argument(arg))
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "unsafe kernel argument",
                ));
            }
        }
        Ok(())
    }

    pub fn render_grub_config(&self) -> String {
        let mut output = String::from(
            r#"# Generated by Void Boot. Review profiles before using on physical media.
set default=0
set timeout=10
set timeout_style=menu
if loadfont $prefix/fonts/unicode.pf2; then
  set gfxmode=auto
  insmod all_video
  insmod gfxterm
  terminal_output gfxterm
fi
insmod part_gpt
insmod part_msdos
insmod fat
insmod ext2
insmod iso9660
insmod ata
insmod loopback
insmod search
insmod search_fs_file
insmod linux
set theme=$prefix/themes/void-boot/theme.txt
"#,
        );

        for profile in &self.profiles {
            if let Some(config_path) = &profile.loopback_config {
                let load_config = if profile.loopback_config_source { "source" } else { "configfile" };
                output.push_str(&format!(
                    r#"
menuentry "{}" --class atdev-linux {{
  set iso_path="/{}"
  export iso_path
  if search --no-floppy --file --set=root "$iso_path"; then
    loopback loop "$iso_path"
    set root=(loop)
    {} /{}
    loopback --delete loop
  else
    echo "ISO not found: $iso_path"
    sleep 5
  fi
}}
"#,
                    profile.name, profile.iso, load_config, config_path
                ));
            } else {
                let mut kernel_args = profile.linux_args.clone();
                if profile.findiso {
                    kernel_args.push("findiso=$isofile".to_string());
                }
                if let Some(argument) = &profile.iso_path_argument {
                    kernel_args.push(format!("{argument}=$isofile"));
                }
                let args = kernel_args.join(" ");
                let linux_command = if args.is_empty() {
                    format!("linux (loop)/{}", profile.kernel)
                } else {
                    format!("linux (loop)/{} {}", profile.kernel, args)
                };
                output.push_str(&format!(
                    r#"
menuentry "{}" --class atdev-linux {{
  set isofile="/{}"
  if search --no-floppy --file --set=root "$isofile"; then
    loopback loop "$isofile"
    {}
    initrd (loop)/{}
  else
    echo "ISO not found: $isofile"
    sleep 5
  fi
}}
"#,
                    profile.name, profile.iso, linux_command, profile.initrd
                ));
            }
        }

        output.push_str(
            r#"
menuentry "Restart computer" --class restart {
  reboot
}
menuentry "Shut down" --class shutdown {
  halt
}
"#,
        );
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ProfileCatalog {
        ProfileCatalog {
            profiles: vec![BootProfile {
                id: "debian-live".into(),
                name: "Debian Live".into(),
                iso: "iso/debian-live.iso".into(),
                kernel: "live/vmlinuz".into(),
                initrd: "live/initrd.img".into(),
                linux_args: vec!["boot=live".into(), "components".into()],
                findiso: true,
                iso_path_argument: None,
                loopback_config: None,
                loopback_config_source: false,
            }],
        }
    }

    #[test]
    fn profile_catalog_round_trips_through_toml_for_gui_editor() {
        let original = sample();
        let encoded = toml::to_string_pretty(&original).expect("serialize profile catalog");
        let decoded: ProfileCatalog =
            toml::from_str(&encoded).expect("deserialize profile catalog");
        decoded
            .validate()
            .expect("validate round-tripped profile catalog");
        assert_eq!(decoded.profiles, original.profiles);
    }

    #[test]
    fn renders_graphical_grub_menu_with_profile_entries() {
        let rendered = sample().render_grub_config();
        assert!(rendered.contains("loadfont $prefix/fonts/unicode.pf2"));
        assert!(rendered.contains("terminal_output gfxterm"));
        assert!(rendered.contains("search --no-floppy --file --set=root \"$isofile\""));
        assert!(rendered.contains("menuentry \"Debian Live\""));
        assert!(rendered.contains("loopback loop \"$isofile\""));
        assert!(
            rendered.contains("linux (loop)/live/vmlinuz boot=live components findiso=$isofile")
        );
        assert!(rendered.contains("initrd (loop)/live/initrd.img"));
        assert!(rendered.contains("Restart computer"));
        assert!(!rendered.contains("fwsetup"));
    }

    #[test]
    fn renders_kernel_command_without_empty_argument_suffix() {
        let mut catalog = sample();
        catalog.profiles[0].linux_args.clear();
        assert!(catalog
            .render_grub_config()
            .contains("    linux (loop)/live/vmlinuz findiso=$isofile\n"));
    }

    #[test]
    fn renders_configurable_iso_path_parameter_for_ubuntu_family() {
        let mut catalog = sample();
        catalog.profiles[0].findiso = false;
        catalog.profiles[0].iso_path_argument = Some("iso-scan/filename".into());
        let rendered = catalog.render_grub_config();
        assert!(rendered.contains("boot=live components iso-scan/filename=$isofile"));
        assert!(!rendered.contains("findiso=$isofile"));
    }

    #[test]
    fn rejects_unsafe_iso_path_kernel_parameter() {
        let mut catalog = sample();
        catalog.profiles[0].findiso = false;
        catalog.profiles[0].iso_path_argument = Some("iso-scan/filename;reboot".into());
        assert!(catalog.validate().is_err());
    }

    #[test]
    fn renders_distribution_provided_arch_loopback_recipe() {
        let mut catalog = sample();
        catalog.profiles[0].loopback_config = Some("boot/grub/loopback.cfg".into());
        let rendered = catalog.render_grub_config();
        assert!(rendered.contains("export iso_path"));
        assert!(rendered.contains("set root=(loop)"));
        assert!(rendered.contains("configfile /boot/grub/loopback.cfg"));
        assert!(!rendered.contains("linux (loop)/live/vmlinuz"));
    }

    #[test]
    fn renders_openSUSE_source_mode_for_embedded_recipe() {
        let mut catalog = sample();
        catalog.profiles[0].loopback_config = Some("boot/grub2/loopback.cfg".into());
        catalog.profiles[0].loopback_config_source = true;
        let rendered = catalog.render_grub_config();
        assert!(rendered.contains("source /boot/grub2/loopback.cfg"));
        assert!(!rendered.contains("configfile /boot/grub2/loopback.cfg"));
    }

    #[test]
    fn rejects_unsafe_embedded_loopback_recipe_path() {
        let mut catalog = sample();
        catalog.profiles[0].loopback_config = Some("../outside.cfg".into());
        assert!(catalog.validate().is_err());
    }

    #[test]
    fn requires_recipe_path_when_source_mode_is_enabled() {
        let mut catalog = sample();
        catalog.profiles[0].loopback_config_source = true;
        assert!(catalog.validate().is_err());
    }

    #[test]
    fn rejects_duplicate_profile_ids() {
        let mut catalog = sample();
        let mut duplicate = catalog.profiles[0].clone();
        duplicate.name = "Second Linux".into();
        catalog.profiles.push(duplicate);
        assert!(catalog.validate().is_err());
    }

    #[test]
    fn rejects_parent_directory_traversal() {
        let mut catalog = sample();
        catalog.profiles[0].kernel = "../outside/vmlinuz".into();
        assert!(catalog.validate().is_err());
    }

    #[test]
    fn rejects_grub_expression_in_kernel_arguments() {
        let mut catalog = sample();
        catalog.profiles[0].linux_args = vec!["quiet;reboot".into()];
        assert!(catalog.validate().is_err());
    }

    #[test]
    fn every_shipped_experimental_profile_catalog_parses_and_validates() {
        let shipped_catalogs = [
            include_str!("../profiles/ubuntu-family-loopback.toml"),
            include_str!("../profiles/archlinux-loopback.toml"),
            include_str!("../profiles/opensuse-tumbleweed-loopback.toml"),
        ];

        let mut all_profiles = Vec::new();
        for contents in shipped_catalogs {
            let catalog: ProfileCatalog =
                toml::from_str(contents).expect("shipped profile TOML must parse");
            catalog.validate().expect("shipped profiles must validate");
            all_profiles.extend(catalog.profiles);
        }

        let combined = ProfileCatalog { profiles: all_profiles };
        combined
            .validate()
            .expect("all shipped profile IDs must be unique and valid");
        assert_eq!(combined.profiles.len(), 6);
    }

    #[test]
    fn shipped_loopback_profiles_render_their_distribution_owned_recipes() {
        let arch: ProfileCatalog = toml::from_str(include_str!(
            "../profiles/archlinux-loopback.toml"
        ))
        .expect("parse Arch profile");
        let suse: ProfileCatalog = toml::from_str(include_str!(
            "../profiles/opensuse-tumbleweed-loopback.toml"
        ))
        .expect("parse openSUSE profile");

        let arch_grub = arch.render_grub_config();
        assert!(arch_grub.contains("search --no-floppy --file --set=root \"$iso_path\""));
        assert!(arch_grub.contains("configfile /boot/grub/loopback.cfg"));

        let suse_grub = suse.render_grub_config();
        assert!(suse_grub.contains("source /boot/grub2/loopback.cfg"));
    }

    #[test]
    fn shipped_ubuntu_flavour_profiles_render_casper_iso_paths() {
        let ubuntu: ProfileCatalog = toml::from_str(include_str!(
            "../profiles/ubuntu-family-loopback.toml"
        ))
        .expect("parse Ubuntu family profiles");
        ubuntu.validate().expect("validate Ubuntu family profiles");

        let grub = ubuntu.render_grub_config();
        for distro in ["Ubuntu", "Lubuntu", "Kubuntu", "Xubuntu"] {
            assert!(grub.contains(&format!("menuentry \"{distro} Live (experimental)\"")));
        }
        assert_eq!(grub.matches("iso-scan/filename=$isofile").count(), 4);
    }

    #[test]
    fn rejects_unsafe_menu_labels() {
        let mut catalog = sample();
        catalog.profiles[0].name = "Linux\"; reboot".into();
        assert!(catalog.validate().is_err());
    }
}
