use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Role an artifact plays when flashing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Whole-disk image (`*.img`).
    Disk,

    /// Bootable ISO image (`*.iso`).
    Iso,

    /// Boot partition image (`*.boot.ext4`).
    Boot,

    /// EFI system partition image (`*.efi.fat32`).
    Efi,

    /// Root partition image (`*.root.ext4`).
    Root,
}

impl Role {
    pub const fn as_str(self) -> &'static str {
        match self {
            Role::Disk => "disk",
            Role::Iso => "iso",
            Role::Boot => "boot",
            Role::Efi => "efi",
            Role::Root => "root",
        }
    }

    /// The role a file name advertises, e.g. `*.boot.ext4`.
    pub fn from_file_name(file_name: &str) -> Option<Self> {
        let name = file_name.to_ascii_lowercase();
        let (stem, extension) = name.rsplit_once('.')?;

        match extension {
            "ext4" if stem.ends_with(".boot") => Some(Role::Boot),
            "ext4" => Some(Role::Root),
            "fat32" => Some(Role::Efi),
            "img" => Some(Role::Disk),
            "iso" => Some(Role::Iso),
            _ => None,
        }
    }

    /// Write-order key: whole-disk artifacts come before partition artifacts.
    pub const fn rank(self) -> u8 {
        match self {
            Role::Disk | Role::Iso => 0,
            Role::Boot | Role::Efi => 1,
            Role::Root => 2,
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A file that gets written to a device, e.g. `deepin-k1-…root.ext4`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Artifact {
    pub path: PathBuf,

    pub role: Role,

    /// Size in bytes.
    pub size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_roles_from_file_names() {
        for (name, role) in [
            ("deepin-jh7110-riscv64-25-desktop-installer.img", Role::Disk),
            ("deepin-riscv64-25-crimson-immutable.iso", Role::Iso),
            (
                "deepin-eic7700-riscv64-25-desktop-installer.boot.ext4",
                Role::Boot,
            ),
            ("deepin-q6a-arm64-25-desktop-installer.efi.fat32", Role::Efi),
            (
                "deepin-k1-riscv64-25-desktop-installer.root.ext4",
                Role::Root,
            ),
            (
                "deepin-rockchip-arm64-25-desktop-installer.ext4",
                Role::Root,
            ),
        ] {
            assert_eq!(Role::from_file_name(name), Some(role), "{name}");
        }

        for name in [
            "deepin-k1-riscv64-25-desktop-installer.sha256sum",
            "deepin-k1-riscv64-25-desktop-installer.md5sum",
            "README.md",
        ] {
            assert_eq!(Role::from_file_name(name), None, "{name}");
        }
    }
}
