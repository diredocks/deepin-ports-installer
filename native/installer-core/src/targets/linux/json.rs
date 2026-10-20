//! `lsblk --json` reader (the preferred backend).

use std::path::PathBuf;

use serde::Deserialize;

use super::{Device, is_virtual, partition_table, run_lsblk, text};
use crate::Result;
use crate::targets::{Mountpoint, Target};

/// Enumerate targets from `lsblk --json`.
pub(super) fn scan() -> Result<Vec<Target>> {
    let output = run_lsblk(&["--bytes", "--all", "--json", "--paths", "--output-all"])?;

    let parsed: LsblkOutput = serde_json::from_str(&output).map_err(crate::Error::LsblkJson)?;
    Ok(parsed.blockdevices.iter().filter_map(target).collect())
}

/// Top-level `lsblk --json` document.
#[derive(Deserialize)]
struct LsblkOutput {
    #[serde(default)]
    blockdevices: Vec<LsblkDevice>,
}

/// One `lsblk --json` block device.
///
/// Every column is optional: which ones `lsblk` emits depends on its version
/// and on `--output-all` support.
#[derive(Deserialize)]
struct LsblkDevice {
    name: Option<String>,
    label: Option<String>,
    partlabel: Option<String>,
    model: Option<String>,
    vendor: Option<String>,
    mountpoint: Option<String>,
    #[serde(default)]
    mountpoints: Vec<String>,
    subsystems: Option<String>,
    tran: Option<String>,
    pttype: Option<String>,
    size: Option<u64>,
    #[serde(rename = "phy-sec")]
    physical_block_size: Option<u64>,
    #[serde(rename = "log-sec")]
    logical_block_size: Option<u64>,
    ro: Option<bool>,
    rm: Option<bool>,
    hotplug: Option<bool>,
    #[serde(default)]
    children: Vec<LsblkDevice>,
}

impl LsblkDevice {
    /// Mount points of this device, looking through nested children (LVM,
    /// LUKS, ...). Rows with children only report what their leaves carry.
    fn mountpoints(&self) -> Vec<Mountpoint> {
        if !self.children.is_empty() {
            return self.children.iter().flat_map(Self::mountpoints).collect();
        }

        let label = text(&self.label)
            .or_else(|| text(&self.partlabel))
            .map(str::to_owned);

        // Newer `lsblk` versions list every mount point in `mountpoints`;
        // `mountpoint` only holds the first one and remains as a fallback.
        let paths: Vec<&str> = if self.mountpoints.is_empty() {
            self.mountpoint.iter().map(String::as_str).collect()
        } else {
            self.mountpoints.iter().map(String::as_str).collect()
        };

        paths
            .into_iter()
            .filter(|path| !path.is_empty())
            .map(|path| Mountpoint {
                path: PathBuf::from(path),
                label: label.clone(),
            })
            .collect()
    }
}

/// Convert one JSON device into a target, dropping irrelevant devices.
fn target(device: &LsblkDevice) -> Option<Target> {
    let mut row = Device::new(device.name.as_deref()?)?;

    row.label = device.label.clone();
    row.vendor = device.vendor.clone();
    row.model = device.model.clone();
    row.size = device.size;
    row.block_size = device.physical_block_size;
    row.logical_block_size = device.logical_block_size;
    row.mountpoints = device.mountpoints();
    row.partition_table = partition_table(device.pttype.as_deref());
    row.is_virtual = is_virtual(device.subsystems.as_deref());
    row.set_transport(device.tran.as_deref());
    row.set_flags(
        device.ro.unwrap_or(false),
        device.rm.unwrap_or(false),
        device.hotplug.unwrap_or(false),
    );

    Some(row.into_target("lsblk:json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::targets::PartitionTable;

    const JSON: &str = r#"{
        "blockdevices": [
            {
                "name": "/dev/sda",
                "label": null,
                "partlabel": null,
                "model": "Ultra USB",
                "vendor": "SanDisk",
                "mountpoint": null,
                "subsystems": "block:scsi:usb:pci",
                "tran": "usb",
                "type": "disk",
                "pttype": "gpt",
                "size": 32000000000,
                "phy-sec": 512,
                "log-sec": 512,
                "ro": false,
                "rm": true,
                "hotplug": true,
                "children": [
                    {
                        "name": "/dev/sda1",
                        "label": "DEEPIN",
                        "mountpoint": "/media/deepin",
                        "type": "part"
                    },
                    {
                        "name": "/dev/sda2",
                        "label": null,
                        "partlabel": "root",
                        "mountpoint": null,
                        "type": "part"
                    }
                ]
            },
            {
                "name": "/dev/nvme0n1",
                "model": "SAMSUNG",
                "mountpoint": null,
                "subsystems": "block:nvme:pci",
                "tran": "nvme",
                "type": "disk",
                "pttype": "gpt",
                "size": 1024209543168,
                "phy-sec": 4096,
                "log-sec": 512,
                "ro": false,
                "rm": false,
                "hotplug": false,
                "children": [
                    {
                        "name": "/dev/nvme0n1p1",
                        "partlabel": "EFI",
                        "mountpoint": "/boot/efi",
                        "type": "part"
                    },
                    {
                        "name": "/dev/nvme0n1p2",
                        "type": "part",
                        "children": [
                            {
                                "name": "/dev/mapper/cryptroot",
                                "mountpoint": "/",
                                "mountpoints": ["/", "/home"],
                                "type": "crypt"
                            }
                        ]
                    }
                ]
            },
            {
                "name": "/dev/loop0",
                "type": "loop",
                "size": 0
            },
            {
                "name": "/dev/sr0",
                "type": "rom",
                "size": 0
            },
            {
                "name": "/dev/ram0",
                "type": "disk",
                "size": 0
            },
            {
                "name": "/dev/dm-0",
                "label": "vg-root",
                "mountpoint": "/",
                "subsystems": "block",
                "type": "lvm",
                "size": 100000000,
                "ro": false,
                "rm": false,
                "children": []
            }
        ]
    }"#;

    /// Parse [`JSON`] into targets.
    fn targets() -> Vec<Target> {
        let output: LsblkOutput = serde_json::from_str(JSON).unwrap();
        output.blockdevices.iter().filter_map(target).collect()
    }

    #[test]
    fn parses_json_devices_and_derived_flags() {
        let targets = targets();
        assert_eq!(targets.len(), 3);

        let usb = &targets[0];
        assert_eq!(usb.device, PathBuf::from("/dev/sda"));
        assert_eq!(usb.raw, PathBuf::from("/dev/sda"));
        assert_eq!(usb.enumerator, "lsblk:json");
        assert_eq!(usb.bus_type, "USB");
        assert_eq!(usb.size, Some(32_000_000_000));
        assert_eq!(usb.block_size, 512);
        assert_eq!(usb.logical_block_size, 512);
        assert_eq!(usb.partition_table, Some(PartitionTable::Gpt));
        assert!(usb.is_removable);
        assert_eq!(usb.is_usb, Some(true));
        assert_eq!(usb.is_scsi, Some(false));
        assert!(!usb.is_system);
        assert!(!usb.is_virtual);
        assert_eq!(usb.description, "SanDisk Ultra USB");
        assert_eq!(
            usb.mountpoints,
            vec![Mountpoint {
                path: PathBuf::from("/media/deepin"),
                label: Some("DEEPIN".into()),
            }]
        );
    }

    #[test]
    fn parses_system_disk() {
        let targets = targets();
        let system = &targets[1];

        assert_eq!(system.size, Some(1_024_209_543_168));
        assert_eq!(system.block_size, 4096);
        assert_eq!(system.logical_block_size, 512);
        assert!(!system.is_removable);
        assert!(system.is_system);
        assert_eq!(system.description, "SAMSUNG");
        assert_eq!(
            system.mountpoints,
            vec![
                Mountpoint {
                    path: PathBuf::from("/boot/efi"),
                    label: Some("EFI".into()),
                },
                Mountpoint {
                    path: PathBuf::from("/"),
                    label: None,
                },
                Mountpoint {
                    path: PathBuf::from("/home"),
                    label: None,
                },
            ]
        );
    }

    #[test]
    fn marks_single_subsystem_devices_virtual() {
        let targets = targets();
        let lvm = targets.last().unwrap();

        assert_eq!(lvm.device, PathBuf::from("/dev/dm-0"));
        assert!(lvm.is_virtual);
        assert!(lvm.is_removable);
        assert!(!lvm.is_system);
        assert_eq!(
            lvm.mountpoints,
            vec![Mountpoint {
                path: PathBuf::from("/"),
                label: Some("vg-root".into()),
            }]
        );
    }
}
