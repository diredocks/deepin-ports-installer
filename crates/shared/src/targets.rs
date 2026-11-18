//! Local block-device enumeration (modeled after balena's `drivelist`).
//!
//! Linux shells out to `lsblk` (JSON with `--pairs` fallback); Windows and
//! macOS are `todo!()` placeholders.

pub use error::{Error, Result};

use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

mod block_device;
mod error;
pub mod fastboot;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PartitionTable {
    Mbr,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum Enumerator {
    #[serde(rename = "lsblk:json")]
    LsblkJson,

    #[serde(rename = "lsblk:pairs")]
    LsblkPairs,

    #[serde(rename = "usb:fastboot")]
    UsbFastboot,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BusType {
    Usb,
    Sata,
    Scsi,
    Ata,
    Ide,
    Pci,
    Unknown(String),
}

impl BusType {
    pub fn unreported() -> Self {
        BusType::Unknown("UNKNOWN".to_owned())
    }

    pub fn is_scsi(&self) -> bool {
        matches!(
            self,
            BusType::Sata | BusType::Scsi | BusType::Ata | BusType::Ide | BusType::Pci
        )
    }

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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Mountpoint {
    pub path: PathBuf,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Target {
    pub enumerator: Enumerator,
    pub bus_type: BusType,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bus_version: Option<String>,

    pub device: PathBuf,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_path: Option<PathBuf>,

    pub raw: PathBuf,
    pub description: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,

    pub block_size: u64,
    pub logical_block_size: u64,

    #[serde(default)]
    pub mountpoints: Vec<Mountpoint>,

    pub is_read_only: bool,
    pub is_system: bool,
    pub is_virtual: bool,
    pub is_removable: bool,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_card: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_uas: Option<bool>,

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
    pub fn new(targets: Vec<Target>) -> Self {
        Self { targets }
    }

    /// Enumerate the local block devices and USB fastboot targets of this host.
    ///
    /// On Linux block devices come from `lsblk`, and USB fastboot devices are
    /// discovered via `nusb`.
    pub fn scan() -> Result<Self> {
        #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
        {
            let mut targets = block_device::scan()?;
            targets.extend(fastboot::scan());
            Ok(Self::new(targets))
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            Err(Error::UnsupportedPlatform)
        }
    }

    /// All targets, in discovery order.
    pub fn targets(&self) -> &[Target] {
        &self.targets
    }

    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(&self.targets)?)
    }

    /// Serialize the targets as a `[[targets]]` TOML document.
    pub fn to_toml(&self) -> Result<String> {
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
        assert_eq!(BusType::from("USB"), BusType::Usb);
        assert_eq!(BusType::from(" sata "), BusType::Sata);
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
}
