//! Enumeration of local block devices usable as flash targets.
//!
//! The data model follows balena's [`drivelist`] package: every target carries
//! enough information for a UI to describe it (vendor/model/labels, size, bus)
//! and for the flash logic to decide whether it is safe to overwrite (system
//! disk, read-only, removable).
//!
//! Linux enumeration shells out to `lsblk`, preferring its JSON output and
//! falling back to the older `--pairs` format, exactly like `drivelist` does.
//! Windows and macOS are scaffolded with [`todo!`] placeholders.
//!
//! [`drivelist`]: https://github.com/balena-io-modules/drivelist

use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::Result;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

/// Partition table scheme found on a target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PartitionTable {
    /// Master Boot Record (a.k.a. DOS) partition table.
    Mbr,

    /// GUID Partition Table.
    Gpt,
}

impl fmt::Display for PartitionTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            PartitionTable::Mbr => "mbr",
            PartitionTable::Gpt => "gpt",
        })
    }
}

/// A mount point of a partition on a target.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Mountpoint {
    /// Mount path, e.g. `/boot/efi` or `[SWAP]`.
    pub path: PathBuf,

    /// Filesystem or partition label, when one is set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// A local block device that can be used as a flash target.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Target {
    /// Discovery backend that produced this target, e.g. `lsblk:json`.
    pub enumerator: String,

    /// Transport bus, e.g. `USB`, `SATA` or `UNKNOWN`.
    pub bus_type: String,

    /// Bus protocol version; only reported by some platforms.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bus_version: Option<String>,

    /// Device node, e.g. `/dev/sdb`.
    pub device: PathBuf,

    /// Stable `/dev/disk/by-path` link, when one exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_path: Option<PathBuf>,

    /// Kernel device path, e.g. `/dev/sdb`.
    pub raw: PathBuf,

    /// Human readable description built from labels, vendor and model.
    pub description: String,

    /// Size in bytes, when the device size is known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,

    /// Physical block size in bytes.
    pub block_size: u64,

    /// Logical block size in bytes.
    pub logical_block_size: u64,

    /// Mount points of the target's partitions.
    #[serde(default)]
    pub mountpoints: Vec<Mountpoint>,

    /// Whether the device is read-only.
    pub is_read_only: bool,

    /// Whether this is the disk the running system lives on.
    pub is_system: bool,

    /// Whether this is a virtual device (LVM, device mapper, ...).
    pub is_virtual: bool,

    /// Whether the device can be removed while the system is running.
    pub is_removable: bool,

    /// Whether this is an SD/MMC card; only reported on some platforms.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_card: Option<bool>,

    /// Whether the device is attached over SCSI/SATA.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_scsi: Option<bool>,

    /// Whether the device is attached over USB.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_usb: Option<bool>,

    /// Whether the device uses USB Attached SCSI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_uas: Option<bool>,

    /// Partition table scheme, when one was detected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partition_table: Option<PartitionTable>,
}

impl Target {
    /// Whether the target is a sensible default for flashing.
    ///
    /// System disks are excluded because writing to them usually destroys the
    /// running system; read-only devices cannot be written at all.
    pub fn is_flashable(&self) -> bool {
        !self.is_system && !self.is_read_only
    }
}

/// The set of local block devices found on this host.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TargetList {
    targets: Vec<Target>,
}

impl TargetList {
    /// Wrap an already collected list of targets.
    pub fn new(targets: Vec<Target>) -> Self {
        Self { targets }
    }

    /// Enumerate the local block devices of this host.
    ///
    /// On Linux this runs `lsblk`; see the module documentation for details.
    pub fn scan() -> Result<Self> {
        #[cfg(target_os = "linux")]
        {
            linux::scan()
        }

        #[cfg(target_os = "macos")]
        {
            macos::scan()
        }

        #[cfg(target_os = "windows")]
        {
            windows::scan()
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            Err(crate::Error::UnsupportedPlatform)
        }
    }

    /// All targets, in discovery order.
    pub fn targets(&self) -> &[Target] {
        &self.targets
    }

    /// Iterate over all targets.
    pub fn iter(&self) -> std::slice::Iter<'_, Target> {
        self.targets.iter()
    }

    /// Number of targets.
    pub fn len(&self) -> usize {
        self.targets.len()
    }

    /// Whether no targets were found.
    pub fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }

    /// Serialize the targets as a pretty-printed JSON array.
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(&self.targets)?)
    }

    /// Serialize the targets as a `[[targets]]` TOML document.
    pub fn to_toml(&self) -> Result<String> {
        /// Wrapper used to emit the listing as a top-level TOML table.
        #[derive(Serialize)]
        struct Document<'a> {
            targets: &'a [Target],
        }

        Ok(toml::to_string_pretty(&Document {
            targets: &self.targets,
        })?)
    }
}

impl From<Vec<Target>> for TargetList {
    fn from(targets: Vec<Target>) -> Self {
        Self { targets }
    }
}

impl<'a> IntoIterator for &'a TargetList {
    type Item = &'a Target;
    type IntoIter = std::slice::Iter<'a, Target>;

    fn into_iter(self) -> Self::IntoIter {
        self.targets.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_target() -> Target {
        Target {
            enumerator: "lsblk:json".into(),
            bus_type: "USB".into(),
            bus_version: None,
            device: PathBuf::from("/dev/sdb"),
            device_path: Some(PathBuf::from(
                "/dev/disk/by-path/pci-0000:00:14.0-usb-0:2:1.0",
            )),
            raw: PathBuf::from("/dev/sdb"),
            description: "SD Card Reader".into(),
            size: Some(32_000_000_000),
            block_size: 512,
            logical_block_size: 512,
            mountpoints: vec![Mountpoint {
                path: PathBuf::from("/media/card"),
                label: Some("DEEPIN".into()),
            }],
            is_read_only: false,
            is_system: false,
            is_virtual: false,
            is_removable: true,
            is_card: None,
            is_scsi: None,
            is_usb: Some(true),
            is_uas: None,
            partition_table: Some(PartitionTable::Gpt),
        }
    }

    #[test]
    fn flashable_excludes_system_and_read_only_targets() {
        let mut target = sample_target();
        assert!(target.is_flashable());

        target.is_system = true;
        assert!(!target.is_flashable());

        target.is_system = false;
        target.is_read_only = true;
        assert!(!target.is_flashable());
    }

    #[test]
    fn to_json_emits_target_array() {
        let list = TargetList::from(vec![sample_target()]);
        let value: serde_json::Value = serde_json::from_str(&list.to_json().unwrap()).unwrap();

        let array = value.as_array().unwrap();
        assert_eq!(array.len(), 1);
        assert_eq!(array[0]["device"], "/dev/sdb");
        assert_eq!(array[0]["is_removable"], true);
        assert_eq!(array[0]["partition_table"], "gpt");
        assert_eq!(array[0]["mountpoints"][0]["path"], "/media/card");
    }

    #[test]
    fn to_toml_round_trips_targets_layout() {
        let list = TargetList::from(vec![sample_target()]);
        let rendered = list.to_toml().unwrap();

        assert!(rendered.contains("[[targets]]"));

        /// Mirror of the `[[targets]]` document produced by [`TargetList`].
        #[derive(Deserialize)]
        struct Document {
            targets: Vec<Target>,
        }

        let parsed: Document = toml::from_str(&rendered).unwrap();
        assert_eq!(parsed.targets, vec![sample_target()]);
    }

    #[test]
    fn empty_list_serializes_to_empty_array() {
        let list = TargetList::default();
        assert!(list.is_empty());
        assert_eq!(list.to_json().unwrap(), "[]");
    }
}
