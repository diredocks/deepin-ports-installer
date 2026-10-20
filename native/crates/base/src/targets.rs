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

use serde::{Deserialize, Deserializer, Serialize, Serializer};

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

/// Discovery backend that produced a [`Target`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum Enumerator {
    /// `lsblk --bytes --all --json --paths --output-all`.
    #[serde(rename = "lsblk:json")]
    LsblkJson,

    /// `lsblk --bytes --all --pairs`, for `lsblk` builds without JSON output.
    #[serde(rename = "lsblk:pairs")]
    LsblkPairs,
}

/// Transport bus a [`Target`] is attached over.
///
/// `lsblk` reports this as `TRAN`; values outside the known set are kept
/// verbatim in [`BusType::Unknown`] instead of being dropped.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BusType {
    /// USB.
    Usb,

    /// Serial ATA.
    Sata,

    /// SCSI.
    Scsi,

    /// Parallel ATA, reported by some old `lsblk` versions.
    Ata,

    /// (E)IDE, reported by some old `lsblk` versions.
    Ide,

    /// PCI/PCIe.
    Pci,

    /// Any other transport, uppercased as `lsblk` was asked to report it.
    Unknown(String),
}

impl BusType {
    /// The bus reported when the backend knows no transport at all.
    pub fn unreported() -> Self {
        BusType::Unknown("UNKNOWN".to_owned())
    }

    /// Whether the bus carries SCSI style commands, mirroring `drivelist`'s
    /// `isSCSI` flag.
    pub fn is_scsi(&self) -> bool {
        matches!(
            self,
            BusType::Sata | BusType::Scsi | BusType::Ata | BusType::Ide | BusType::Pci
        )
    }

    /// Whether the bus is USB, mirroring `drivelist`'s `isUSB` flag.
    pub fn is_usb(&self) -> bool {
        matches!(self, BusType::Usb)
    }
}

/// Normalize an `lsblk` `TRAN` value.
///
/// Values outside the known set are kept (uppercased) instead of being folded
/// into `UNKNOWN`, so `nvme`/`sas`/`mmc` stay distinguishable. Empty values are
/// printed by `lsblk` itself (`"tran": ""`) and can appear in hand-written
/// documents; both mean "no transport", like a missing column.
impl From<&str> for BusType {
    fn from(transport: &str) -> Self {
        match transport.trim().to_ascii_lowercase().as_str() {
            "usb" => BusType::Usb,
            "sata" => BusType::Sata,
            "scsi" => BusType::Scsi,
            "ata" => BusType::Ata,
            "ide" => BusType::Ide,
            "pci" => BusType::Pci,
            "" => BusType::unreported(),
            other => BusType::Unknown(other.to_ascii_uppercase()),
        }
    }
}

impl fmt::Display for BusType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            BusType::Usb => "USB",
            BusType::Sata => "SATA",
            BusType::Scsi => "SCSI",
            BusType::Ata => "ATA",
            BusType::Ide => "IDE",
            BusType::Pci => "PCI",
            BusType::Unknown(transport) => transport,
        })
    }
}

impl Serialize for BusType {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for BusType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        Ok(BusType::from(String::deserialize(deserializer)?.as_str()))
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
    /// Discovery backend that produced this target.
    pub enumerator: Enumerator,

    /// Transport bus, e.g. [`BusType::Usb`] or [`BusType::unreported`].
    pub bus_type: BusType,

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
    /// running system, read-only devices cannot be written at all, and
    /// non-removable devices are internal drives rather than flash media.
    pub fn is_flashable(&self) -> bool {
        !self.is_system && !self.is_read_only && self.is_removable
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

impl IntoIterator for TargetList {
    type Item = Target;
    type IntoIter = std::vec::IntoIter<Target>;

    fn into_iter(self) -> Self::IntoIter {
        self.targets.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_target() -> Target {
        Target {
            enumerator: Enumerator::LsblkJson,
            bus_type: BusType::Usb,
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
            is_uas: None,
            partition_table: Some(PartitionTable::Gpt),
        }
    }

    #[test]
    fn flashable_excludes_system_read_only_and_fixed_targets() {
        let mut target = sample_target();
        assert!(target.is_flashable());

        target.is_system = true;
        assert!(!target.is_flashable());

        target.is_system = false;
        target.is_read_only = true;
        assert!(!target.is_flashable());

        target.is_read_only = false;
        target.is_removable = false;
        assert!(!target.is_flashable());
    }

    #[test]
    fn to_json_emits_target_array() {
        let list = TargetList::new(vec![sample_target()]);
        let value: serde_json::Value = serde_json::from_str(&list.to_json().unwrap()).unwrap();

        let array = value.as_array().unwrap();
        assert_eq!(array.len(), 1);
        assert_eq!(array[0]["device"], "/dev/sdb");
        assert_eq!(array[0]["enumerator"], "lsblk:json");
        assert_eq!(array[0]["bus_type"], "USB");
        assert_eq!(array[0]["is_removable"], true);
        assert_eq!(array[0]["partition_table"], "gpt");
        assert_eq!(array[0]["mountpoints"][0]["path"], "/media/card");
        assert!(array[0].get("is_usb").is_none());
        assert!(array[0].get("is_scsi").is_none());
    }

    #[test]
    fn maps_known_transports_to_bus_types() {
        assert_eq!(BusType::from("usb"), BusType::Usb);
        assert_eq!(BusType::from("USB"), BusType::Usb);
        assert_eq!(BusType::from(" sata "), BusType::Sata);
        assert_eq!(BusType::from("scsi"), BusType::Scsi);
        assert_eq!(BusType::from("ata"), BusType::Ata);
        assert_eq!(BusType::from("ide"), BusType::Ide);
        assert_eq!(BusType::from("pci"), BusType::Pci);
        assert_eq!(BusType::from(""), BusType::unreported());
    }

    #[test]
    fn keeps_unknown_transports_verbatim() {
        assert_eq!(BusType::from("nvme"), BusType::Unknown("NVME".into()));
        assert_eq!(BusType::from("sas"), BusType::Unknown("SAS".into()));
        assert_eq!(BusType::Unknown("NVME".into()).to_string(), "NVME");
        assert_eq!(BusType::unreported().to_string(), "UNKNOWN");
    }

    #[test]
    fn classifies_scsi_buses() {
        assert!(BusType::Usb.is_usb());
        assert!(!BusType::Usb.is_scsi());
        assert!(BusType::Sata.is_scsi());
        assert!(BusType::Pci.is_scsi());
        assert!(!BusType::unreported().is_scsi());
        assert!(!BusType::Unknown("SAS".into()).is_usb());
    }

    #[test]
    fn bus_type_and_enumerator_round_trip_through_serde() {
        for bus in [
            BusType::Usb,
            BusType::Sata,
            BusType::Unknown("NVME".into()),
            BusType::unreported(),
        ] {
            let json = serde_json::to_string(&bus).unwrap();
            assert_eq!(serde_json::from_str::<BusType>(&json).unwrap(), bus);
        }

        let json = serde_json::to_string(&Enumerator::LsblkPairs).unwrap();
        assert_eq!(json, "\"lsblk:pairs\"");
        assert_eq!(
            serde_json::from_str::<Enumerator>(&json).unwrap(),
            Enumerator::LsblkPairs
        );
    }

    #[test]
    fn to_toml_round_trips_targets_layout() {
        let list = TargetList::new(vec![sample_target()]);
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
