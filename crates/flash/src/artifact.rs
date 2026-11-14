//! Image artifacts found in an archive or on disk.
//!
//! An artifact is what gets written to the target device, which is why the
//! role of a file is read from its name, following the build naming convention
//! (`<stem>.boot.ext4`, `<stem>.efi.fat32`, `<stem>.root.ext4`): a checksum
//! manifest says nothing about which file is a boot partition or in which order
//! partitions are written.

use std::cmp::Ordering;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::Result;

/// Role an artifact plays when flashing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    /// The role advertised by a file name, when it advertises one.
    ///
    /// A bare `*.ext4` counts as a root partition: builds that drop the `root`
    /// marker are still root images.
    pub(crate) fn from_file_name(file_name: &str) -> Option<Self> {
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

    /// Sort key that groups whole-disk artifacts before partition artifacts.
    const fn rank(self) -> u8 {
        match self {
            Role::Disk | Role::Iso => 0,
            Role::Boot | Role::Efi => 1,
            Role::Root => 2,
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Role::Disk => "disk",
            Role::Iso => "iso",
            Role::Boot => "boot",
            Role::Efi => "efi",
            Role::Root => "root",
        })
    }
}

/// A file that will be written to a block device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    /// Path of the file.
    pub path: PathBuf,

    /// What the file is written as.
    pub role: Role,

    /// Size in bytes.
    pub size: u64,
}

impl Artifact {
    /// Detect the artifact in `path`; `None` when the file is not an image.
    pub(crate) fn detect(path: &Path) -> Result<Option<Self>> {
        let role = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(Role::from_file_name);

        let Some(role) = role else {
            return Ok(None);
        };

        let metadata = fsx::metadata(path)?;

        if !metadata.is_file() {
            return Ok(None);
        }

        Ok(Some(Artifact {
            path: path.to_path_buf(),
            role,
            size: metadata.len(),
        }))
    }

    /// Sort artifacts so whole-disk images come before partitions.
    pub(crate) fn order(left: &Self, right: &Self) -> Ordering {
        left.role
            .rank()
            .cmp(&right.role.rank())
            .then_with(|| left.path.cmp(&right.path))
    }
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
    }

    #[test]
    fn ignores_other_files() {
        for name in [
            "deepin-k1-riscv64-25-desktop-installer.sha256sum",
            "deepin-k1-riscv64-25-desktop-installer.md5sum",
            "README.md",
        ] {
            assert_eq!(Role::from_file_name(name), None, "{name}");
        }
    }

    #[test]
    fn sorts_whole_disk_images_first() {
        let mut artifacts = [
            artifact("/x/root.ext4", Role::Root),
            artifact("/x/boot.ext4", Role::Boot),
            artifact("/x/disk.img", Role::Disk),
        ];

        artifacts.sort_by(Artifact::order);

        let roles: Vec<Role> = artifacts.iter().map(|artifact| artifact.role).collect();
        assert_eq!(roles, [Role::Disk, Role::Boot, Role::Root]);
    }

    fn artifact(path: &str, role: Role) -> Artifact {
        Artifact {
            path: PathBuf::from(path),
            role,
            size: 1,
        }
    }
}
