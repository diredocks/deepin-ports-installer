use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::catalog::{Error, Result};

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

    /// MaskROM loader binary (`rkdeveloptool db <loader>`).
    Loader,

    /// Bootinfo binary (`fastboot flash bootinfo <bootinfo>`).
    Bootinfo,

    /// First-stage bootloader (`fastboot flash fsbl <fsbl>`).
    Fsbl,

    /// U-Boot environment image (`fastboot flash env <env>`).
    Env,

    /// OpenSBI firmware image (`fastboot flash opensbi <opensbi>`).
    Opensbi,

    /// Allwinner stage-1 SPL bootloader (`boot0_sdcard.bin`).
    Boot0,

    /// U-Boot binary or FIT image (`u-boot-with-spl.bin`, `u-boot.itb`, etc.).
    Uboot,
}

impl Role {
    pub const fn as_str(self) -> &'static str {
        match self {
            Role::Disk => "disk",
            Role::Iso => "iso",
            Role::Boot => "boot",
            Role::Efi => "efi",
            Role::Root => "root",
            Role::Loader => "loader",
            Role::Bootinfo => "bootinfo",
            Role::Fsbl => "fsbl",
            Role::Env => "env",
            Role::Opensbi => "opensbi",
            Role::Boot0 => "boot0",
            Role::Uboot => "uboot",
        }
    }

    /// Whether this role represents a user-supplied extra file rather than an
    /// image artifact unpacked from an archive.
    pub const fn is_extra(self) -> bool {
        matches!(
            self,
            Role::Loader
                | Role::Bootinfo
                | Role::Fsbl
                | Role::Env
                | Role::Opensbi
                | Role::Boot0
                | Role::Uboot
        )
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

    /// Write-order key: extra files come before whole-disk and partition artifacts.
    pub const fn rank(self) -> u8 {
        match self {
            Role::Loader => 0,
            Role::Bootinfo => 1,
            Role::Fsbl => 2,
            Role::Env => 3,
            Role::Opensbi => 4,
            Role::Boot0 => 5,
            Role::Uboot => 6,
            Role::Disk | Role::Iso => 7,
            Role::Boot | Role::Efi => 8,
            Role::Root => 9,
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Role {
    type Err = Error;

    fn from_str(slug: &str) -> Result<Self> {
        match slug.to_ascii_lowercase().as_str() {
            "disk" => Ok(Role::Disk),
            "iso" => Ok(Role::Iso),
            "boot" => Ok(Role::Boot),
            "efi" => Ok(Role::Efi),
            "root" => Ok(Role::Root),
            "loader" => Ok(Role::Loader),
            "bootinfo" => Ok(Role::Bootinfo),
            "fsbl" => Ok(Role::Fsbl),
            "env" => Ok(Role::Env),
            "opensbi" => Ok(Role::Opensbi),
            "boot0" => Ok(Role::Boot0),
            "uboot" => Ok(Role::Uboot),
            _ => Err(Error::UnknownRole(slug.to_owned())),
        }
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

impl Artifact {
    pub fn by_role(artifacts: &[Self], role: Role) -> Result<&Self> {
        artifacts
            .iter()
            .find(|artifact| artifact.role == role)
            .ok_or(Error::MissingArtifact(role))
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

        for name in [
            "deepin-k1-riscv64-25-desktop-installer.sha256sum",
            "deepin-k1-riscv64-25-desktop-installer.md5sum",
            "README.md",
        ] {
            assert_eq!(Role::from_file_name(name), None, "{name}");
        }
    }
}
