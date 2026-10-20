//! Linux target enumeration via `lsblk`.
//!
//! This is a port of `drivelist`'s `lib/lsblk` helpers:
//!
//! * prefer `lsblk --bytes --all --json --paths --output-all`, falling back to
//!   the older `--pairs` format when the installed `lsblk` does not know JSON
//!   (or prints something unexpected);
//! * expose the same derived flags (`is_virtual`, `is_removable`, `is_system`,
//!   ...) that `drivelist` computes;
//! * resolve `/dev/disk/by-path` links on a best-effort basis.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;
use serde::de::Error as _;

use super::{Mountpoint, PartitionTable, Target, TargetList};
use crate::{Error, Result};

/// The `lsblk` binary.
const LSBLK: &str = "lsblk";

/// Directory holding stable device links.
const DISK_PATH_DIR: &str = "/dev/disk/by-path";

/// Enumerate local block devices.
pub(super) fn scan() -> Result<TargetList> {
    let mut targets = match lsblk_json() {
        Ok(targets) => targets,
        Err(_) => lsblk_pairs()?,
    };

    add_device_paths(&mut targets);
    Ok(TargetList::from(targets))
}

// --- lsblk invocation -------------------------------------------------------

/// Run `lsblk` with `args` and return its stdout.
fn run_lsblk(args: &[&str]) -> Result<String> {
    let output = Command::new(LSBLK)
        .args(args)
        .output()
        .map_err(|source| Error::Command {
            program: LSBLK.to_owned(),
            source,
        })?;

    if !output.status.success() {
        return Err(Error::CommandFailed {
            program: LSBLK.to_owned(),
            status: output.status,
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Enumerate targets from `lsblk --json`.
fn lsblk_json() -> Result<Vec<Target>> {
    let output = run_lsblk(&["--bytes", "--all", "--json", "--paths", "--output-all"])?;

    let parsed: LsblkOutput = serde_json::from_str(&output).map_err(Error::LsblkJson)?;
    Ok(parsed
        .blockdevices
        .iter()
        .filter_map(target_from_json)
        .collect())
}

/// Enumerate targets from `lsblk --pairs`.
fn lsblk_pairs() -> Result<Vec<Target>> {
    let output = run_lsblk(&["--bytes", "--all", "--pairs", "-o", "+pttype"])
        .or_else(|_| run_lsblk(&["--bytes", "--all", "--pairs"]))?;

    parse_pairs(&output)
}

// --- lsblk JSON -------------------------------------------------------------

/// Top-level `lsblk --json` document.
#[derive(Debug, Deserialize)]
struct LsblkOutput {
    #[serde(default)]
    blockdevices: Vec<LsblkDevice>,
}

/// One `lsblk --json` block device.
#[derive(Debug, Default, Deserialize)]
struct LsblkDevice {
    #[serde(default)]
    name: Option<String>,

    #[serde(default)]
    kname: Option<String>,

    #[serde(default)]
    label: Option<String>,

    #[serde(default)]
    partlabel: Option<String>,

    #[serde(default)]
    model: Option<String>,

    #[serde(default)]
    vendor: Option<String>,

    #[serde(default)]
    mountpoint: Option<String>,

    #[serde(default)]
    mountpoints: Vec<String>,

    #[serde(default)]
    subsystems: Option<String>,

    #[serde(default)]
    tran: Option<String>,

    #[serde(default)]
    pttype: Option<String>,

    #[serde(default, deserialize_with = "de_optional_u64")]
    size: Option<u64>,

    #[serde(default, rename = "phy-sec", deserialize_with = "de_optional_u64")]
    physical_block_size: Option<u64>,

    #[serde(default, rename = "log-sec", deserialize_with = "de_optional_u64")]
    logical_block_size: Option<u64>,

    #[serde(default, deserialize_with = "de_optional_bool")]
    ro: Option<bool>,

    #[serde(default, deserialize_with = "de_optional_bool")]
    rm: Option<bool>,

    #[serde(default, deserialize_with = "de_optional_bool")]
    hotplug: Option<bool>,

    #[serde(default)]
    children: Vec<LsblkDevice>,
}

/// A JSON scalar that may be a number or a numeric string.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Numberish {
    Unsigned(u64),
    Signed(i64),
    Text(String),
}

/// A JSON scalar that may be a boolean, a number or a boolean-ish string.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Boolish {
    Bool(bool),
    Number(u8),
    Text(String),
}

impl Boolish {
    /// Interpret the scalar as a flag.
    fn is_true(&self) -> bool {
        match self {
            Boolish::Bool(value) => *value,
            Boolish::Number(value) => *value != 0,
            Boolish::Text(value) => text_is_true(value),
        }
    }
}

/// Deserialize an optional integer, accepting numbers and numeric strings.
fn de_optional_u64<'de, D>(deserializer: D) -> std::result::Result<Option<u64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match Option::<Numberish>::deserialize(deserializer)? {
        None => Ok(None),
        Some(Numberish::Unsigned(value)) => Ok(Some(value)),
        Some(Numberish::Signed(value)) => u64::try_from(value).map(Some).map_err(D::Error::custom),
        Some(Numberish::Text(value)) => {
            let value = value.trim();
            if value.is_empty() {
                Ok(None)
            } else {
                value.parse().map(Some).map_err(D::Error::custom)
            }
        }
    }
}

/// Deserialize an optional flag, accepting booleans, numbers and strings.
fn de_optional_bool<'de, D>(deserializer: D) -> std::result::Result<Option<bool>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<Boolish>::deserialize(deserializer)?.map(|value| value.is_true()))
}

/// Whether a textual flag (`"0"`, `"1"`, `"true"`, ...) is set.
fn text_is_true(value: &str) -> bool {
    !matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "" | "0" | "false" | "no" | "off"
    )
}

/// Convert one JSON device into a target, dropping irrelevant devices.
fn target_from_json(device: &LsblkDevice) -> Option<Target> {
    let name = device.name.as_deref()?;
    let device_path = resolve_device(name);
    if is_ignored(&device_path) {
        return None;
    }

    let raw = device
        .kname
        .as_deref()
        .map(resolve_device)
        .unwrap_or_else(|| device_path.clone());

    let mountpoints = collect_mountpoints(device);

    let is_virtual = device
        .subsystems
        .as_deref()
        .is_some_and(|subsystems| subsystems.eq_ignore_ascii_case("block"));
    let transport = device.tran.as_deref().map(str::to_ascii_lowercase);
    let is_scsi = transport
        .as_deref()
        .map(|tran| matches!(tran, "sata" | "scsi" | "ata" | "ide" | "pci"));
    let is_usb = transport.as_deref().map(|tran| tran == "usb");
    let is_read_only = device.ro.unwrap_or(false);
    let is_removable = device.rm.unwrap_or(false) || device.hotplug.unwrap_or(false) || is_virtual;

    Some(Target {
        enumerator: "lsblk:json".to_owned(),
        bus_type: transport
            .map(|tran| tran.to_ascii_uppercase())
            .unwrap_or_else(|| "UNKNOWN".to_owned()),
        bus_version: None,
        device: device_path,
        device_path: None,
        raw,
        description: describe(device, &mountpoints),
        size: device.size,
        block_size: device.physical_block_size.unwrap_or(512),
        logical_block_size: device.logical_block_size.unwrap_or(512),
        mountpoints,
        is_read_only,
        is_system: !is_removable && !is_virtual,
        is_virtual,
        is_removable,
        is_card: None,
        is_scsi,
        is_usb,
        is_uas: None,
        partition_table: partition_table(device.pttype.as_deref()),
    })
}

// --- lsblk --pairs ----------------------------------------------------------

/// Parse and consolidate an `lsblk --pairs` listing.
fn parse_pairs(output: &str) -> Result<Vec<Target>> {
    let devices: Vec<HashMap<String, String>> = output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(parse_pairs_line)
        .collect::<Result<_>>()?;

    Ok(devices
        .iter()
        .filter(|device| {
            device.get("type").is_some_and(|kind| kind == "disk")
                && !device
                    .get("name")
                    .is_some_and(|name| name.starts_with("ram") || name.starts_with("sr"))
        })
        .filter_map(|device| target_from_pairs(device, &devices))
        .collect())
}

/// Parse a single `KEY="value" KEY="value"` line.
fn parse_pairs_line(line: &str) -> Result<HashMap<String, String>> {
    let mut fields = HashMap::new();
    let mut rest = line.trim();

    while !rest.is_empty() {
        let Some((key, value)) = rest.split_once('=') else {
            return Err(Error::LsblkOutput(line.to_owned()));
        };

        let key = key.trim().to_ascii_lowercase();
        let Some((parsed, consumed)) = parse_quoted(value) else {
            return Err(Error::LsblkOutput(line.to_owned()));
        };

        fields.insert(key, parsed);
        rest = value_rest(value, consumed);
    }

    Ok(fields)
}

/// Parse a leading quoted value, returning it and the number of bytes read.
fn parse_quoted(input: &str) -> Option<(String, usize)> {
    let mut chars = input.char_indices();
    let (_, opening) = chars.next()?;
    if opening != '"' {
        return None;
    }

    let mut value = String::new();
    let mut escaped = false;
    for (offset, ch) in chars {
        if escaped {
            value.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some((value, offset + ch.len_utf8()));
        } else {
            value.push(ch);
        }
    }

    None
}

/// The remainder of `input` after a leading quoted value.
fn value_rest(input: &str, consumed: usize) -> &str {
    input[consumed..].trim_start()
}

/// Convert one consolidated pairs device into a target.
fn target_from_pairs(
    device: &HashMap<String, String>,
    all: &[HashMap<String, String>],
) -> Option<Target> {
    let name = device.get("name")?;
    let device_path = resolve_device(name);
    if is_ignored(&device_path) {
        return None;
    }

    let mountpoints: Vec<Mountpoint> = all
        .iter()
        .filter(|child| {
            child
                .get("type")
                .is_some_and(|kind| matches!(kind.as_str(), "disk" | "part"))
                && child
                    .get("name")
                    .is_some_and(|child_name| child_name.starts_with(name))
        })
        .filter_map(mountpoint_from_pairs)
        .collect();

    let is_virtual = device
        .get("subsystems")
        .is_some_and(|subsystems| subsystems.eq_ignore_ascii_case("block"));
    let transport = device.get("tran").map(|tran| tran.to_ascii_lowercase());
    let is_scsi = transport
        .as_deref()
        .map(|tran| matches!(tran, "sata" | "scsi" | "ata" | "ide" | "pci"));
    let is_usb = transport.as_deref().map(|tran| tran == "usb");
    let is_read_only = device.get("ro").is_some_and(|value| text_is_true(value));
    let is_removable = device.get("rm").is_some_and(|value| text_is_true(value))
        || device
            .get("hotplug")
            .is_some_and(|value| text_is_true(value))
        || is_virtual;

    Some(Target {
        enumerator: "lsblk:pairs".to_owned(),
        bus_type: transport
            .map(|tran| tran.to_ascii_uppercase())
            .unwrap_or_else(|| "UNKNOWN".to_owned()),
        bus_version: None,
        device: device_path,
        device_path: None,
        raw: resolve_device(name),
        description: describe_pairs(device, &mountpoints),
        size: device
            .get("size")
            .and_then(|value| value.trim().parse().ok()),
        block_size: device
            .get("phy-sec")
            .and_then(|value| value.trim().parse().ok())
            .unwrap_or(512),
        logical_block_size: device
            .get("log-sec")
            .and_then(|value| value.trim().parse().ok())
            .unwrap_or(512),
        mountpoints,
        is_read_only,
        is_system: !is_removable && !is_virtual,
        is_virtual,
        is_removable,
        is_card: None,
        is_scsi,
        is_usb,
        is_uas: None,
        partition_table: partition_table(device.get("pttype").map(String::as_str)),
    })
}

/// Build a mount point from a pairs device, when it has one.
fn mountpoint_from_pairs(device: &HashMap<String, String>) -> Option<Mountpoint> {
    let path = device.get("mountpoint").filter(|path| !path.is_empty())?;

    Some(Mountpoint {
        path: PathBuf::from(path),
        label: device
            .get("label")
            .filter(|label| !label.is_empty())
            .or_else(|| device.get("partlabel").filter(|label| !label.is_empty()))
            .cloned(),
    })
}

// --- shared helpers ---------------------------------------------------------

/// Mount points of a device, looking through nested children (LVM, LUKS, ...).
fn collect_mountpoints(device: &LsblkDevice) -> Vec<Mountpoint> {
    if device.children.is_empty() {
        return mountpoints(std::slice::from_ref(device));
    }

    let mut mountpoints = Vec::new();
    for child in &device.children {
        mountpoints.extend(collect_mountpoints(child));
    }
    mountpoints
}

/// Mount points of the given partitions/disks.
fn mountpoints(devices: &[LsblkDevice]) -> Vec<Mountpoint> {
    let mut mountpoints = Vec::new();

    for device in devices {
        let label = device
            .label
            .as_deref()
            .filter(|label| !label.is_empty())
            .or_else(|| {
                device
                    .partlabel
                    .as_deref()
                    .filter(|label| !label.is_empty())
            })
            .map(str::to_owned);

        // Newer `lsblk` versions list every mount point in `mountpoints`;
        // `mountpoint` only holds the first one and remains as a fallback.
        let paths: Vec<&str> = if device.mountpoints.is_empty() {
            device
                .mountpoint
                .as_deref()
                .filter(|path| !path.is_empty())
                .into_iter()
                .collect()
        } else {
            device
                .mountpoints
                .iter()
                .map(String::as_str)
                .filter(|path| !path.is_empty())
                .collect()
        };

        for path in paths {
            mountpoints.push(Mountpoint {
                path: PathBuf::from(path),
                label: label.clone(),
            });
        }
    }

    mountpoints
}

/// Build the human readable description of a JSON device.
fn describe(device: &LsblkDevice, mountpoints: &[Mountpoint]) -> String {
    let name = device.name.as_deref().unwrap_or_default();
    describe_parts(
        device.label.as_deref(),
        device.vendor.as_deref(),
        device.model.as_deref(),
        mountpoints,
        name,
    )
}

/// Build the human readable description of a pairs device.
fn describe_pairs(device: &HashMap<String, String>, mountpoints: &[Mountpoint]) -> String {
    describe_parts(
        device.get("label").map(String::as_str),
        device.get("vendor").map(String::as_str),
        device.get("model").map(String::as_str),
        mountpoints,
        device.get("name").map(String::as_str).unwrap_or_default(),
    )
}

/// Join labels, vendor and model, appending the labels of mounted partitions.
fn describe_parts(
    label: Option<&str>,
    vendor: Option<&str>,
    model: Option<&str>,
    mountpoints: &[Mountpoint],
    fallback: &str,
) -> String {
    let mut parts: Vec<String> = [label, vendor, model]
        .into_iter()
        .flatten()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect();

    let mut sublabels: Vec<String> = Vec::new();
    for mountpoint in mountpoints {
        let sublabel = mountpoint
            .label
            .as_deref()
            .filter(|label| !label.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| mountpoint.path.display().to_string());

        if !sublabels.contains(&sublabel) {
            sublabels.push(sublabel);
        }
    }
    if !sublabels.is_empty() {
        parts.push(format!("({})", sublabels.join(", ")));
    }

    let description = parts
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    if description.is_empty() {
        fallback.to_owned()
    } else {
        description
    }
}

/// The device path of an `lsblk` name, resolving relative names under `/dev`.
fn resolve_device(name: &str) -> PathBuf {
    let path = Path::new(name);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        Path::new("/dev").join(path)
    }
}

/// Whether the device is a loop device, optical drive or RAM disk.
fn is_ignored(device: &Path) -> bool {
    let device = device.to_string_lossy();
    device.starts_with("/dev/loop")
        || device.starts_with("/dev/sr")
        || device.starts_with("/dev/ram")
}

/// Map an `lsblk` partition-table type to [`PartitionTable`].
fn partition_table(pttype: Option<&str>) -> Option<PartitionTable> {
    let pttype = pttype?.trim();
    if pttype.eq_ignore_ascii_case("gpt") {
        Some(PartitionTable::Gpt)
    } else if pttype.eq_ignore_ascii_case("dos") {
        Some(PartitionTable::Mbr)
    } else {
        None
    }
}

/// Attach `/dev/disk/by-path` links to the matching targets.
fn add_device_paths(targets: &mut [Target]) {
    let Ok(entries) = std::fs::read_dir(DISK_PATH_DIR) else {
        return;
    };

    let mut mapping: HashMap<PathBuf, PathBuf> = HashMap::new();
    for entry in entries.flatten() {
        let link_path = entry.path();
        if let Ok(resolved) = std::fs::canonicalize(&link_path) {
            mapping.insert(resolved, link_path);
        }
    }

    for target in targets {
        target.device_path = mapping.get(&target.device).cloned();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const JSON: &str = r#"{
        "blockdevices": [
            {
                "name": "/dev/sda",
                "kname": "/dev/sda",
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
                "kname": "/dev/nvme0n1",
                "model": "SAMSUNG",
                "mountpoint": null,
                "subsystems": "block:nvme:pci",
                "tran": "nvme",
                "type": "disk",
                "pttype": "gpt",
                "size": "1024209543168",
                "phy-sec": "4096",
                "log-sec": "512",
                "ro": "0",
                "rm": "0",
                "hotplug": "0",
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
                "kname": "/dev/dm-0",
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

    #[test]
    fn parses_json_devices_and_derived_flags() {
        let output: LsblkOutput = serde_json::from_str(JSON).unwrap();
        let targets: Vec<Target> = output
            .blockdevices
            .iter()
            .filter_map(target_from_json)
            .collect();

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
        assert_eq!(usb.description, "SanDisk Ultra USB (DEEPIN)");
        assert_eq!(
            usb.mountpoints,
            vec![Mountpoint {
                path: PathBuf::from("/media/deepin"),
                label: Some("DEEPIN".into()),
            }]
        );
    }

    #[test]
    fn parses_numeric_strings_and_system_disk() {
        let output: LsblkOutput = serde_json::from_str(JSON).unwrap();
        let targets: Vec<Target> = output
            .blockdevices
            .iter()
            .filter_map(target_from_json)
            .collect();

        let system = &targets[1];
        assert_eq!(system.size, Some(1_024_209_543_168));
        assert_eq!(system.block_size, 4096);
        assert_eq!(system.logical_block_size, 512);
        assert!(!system.is_removable);
        assert!(system.is_system);
        assert_eq!(system.description, "SAMSUNG (EFI, /, /home)");
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
        let output: LsblkOutput = serde_json::from_str(JSON).unwrap();
        let targets: Vec<Target> = output
            .blockdevices
            .iter()
            .filter_map(target_from_json)
            .collect();

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

    #[test]
    fn ignores_loop_optical_and_ram_devices() {
        for name in ["/dev/loop3", "/dev/sr0", "/dev/ram1"] {
            assert!(is_ignored(Path::new(name)));
        }
        for name in ["/dev/sda", "/dev/nvme0n1", "/dev/mmcblk0"] {
            assert!(!is_ignored(Path::new(name)));
        }
    }

    #[test]
    fn resolves_relative_device_names_under_dev() {
        assert_eq!(resolve_device("sdb"), PathBuf::from("/dev/sdb"));
        assert_eq!(resolve_device("/dev/sdb"), PathBuf::from("/dev/sdb"));
    }

    #[test]
    fn maps_partition_table_types() {
        assert_eq!(partition_table(Some("gpt")), Some(PartitionTable::Gpt));
        assert_eq!(partition_table(Some("GPT")), Some(PartitionTable::Gpt));
        assert_eq!(partition_table(Some("dos")), Some(PartitionTable::Mbr));
        assert_eq!(partition_table(Some("")), None);
        assert_eq!(partition_table(None), None);
    }

    const PAIRS: &str = r#"NAME="sda" MAJ:MIN="8:0" RM="0" SIZE="240065183744" RO="0" TYPE="disk" MOUNTPOINT="" PTTYPE="gpt"
NAME="sda1" MAJ:MIN="8:1" RM="0" SIZE="209715200" RO="0" TYPE="part" MOUNTPOINT="/boot/efi" LABEL="EFI" PTTYPE="gpt"
NAME="sda2" MAJ:MIN="8:2" RM="0" SIZE="115835142144" RO="0" TYPE="part" MOUNTPOINT="" PTTYPE="gpt"
NAME="sdb" MAJ:MIN="8:16" RM="1" SIZE="31457280000" RO="0" TYPE="disk" MOUNTPOINT="" PTTYPE="" TRAN="usb" VENDOR="SanDisk" MODEL="Ultra USB"
NAME="sdb1" MAJ:MIN="8:17" RM="1" SIZE="31457279000" RO="0" TYPE="part" MOUNTPOINT="/media/card" LABEL="DEEPIN" PTTYPE=""
NAME="sr0" MAJ:MIN="11:0" RM="1" SIZE="1048576" RO="0" TYPE="rom" MOUNTPOINT=""
"#;

    #[test]
    fn parses_pairs_output() {
        let targets = parse_pairs(PAIRS).unwrap();
        assert_eq!(targets.len(), 2);

        let system = &targets[0];
        assert_eq!(system.device, PathBuf::from("/dev/sda"));
        assert_eq!(system.enumerator, "lsblk:pairs");
        assert_eq!(system.size, Some(240_065_183_744));
        assert_eq!(system.partition_table, Some(PartitionTable::Gpt));
        assert!(system.is_system);
        assert!(!system.is_removable);
        assert_eq!(system.description, "(EFI)");
        assert_eq!(system.mountpoints.len(), 1);

        let usb = &targets[1];
        assert_eq!(usb.device, PathBuf::from("/dev/sdb"));
        assert_eq!(usb.bus_type, "USB");
        assert_eq!(usb.is_usb, Some(true));
        assert!(usb.is_removable);
        assert!(!usb.is_system);
        assert_eq!(usb.description, "SanDisk Ultra USB (DEEPIN)");
        assert_eq!(usb.mountpoints[0].path, PathBuf::from("/media/card"));
    }

    #[test]
    fn parses_escaped_quotes_in_pairs_values() {
        let line = r#"NAME="sda" MODEL="Some \"Quoted\" Model" EMPTY="" TYPE="disk""#;
        let fields = parse_pairs_line(line).unwrap();

        assert_eq!(fields["model"], r#"Some "Quoted" Model"#);
        assert_eq!(fields["empty"], "");
        assert_eq!(fields["type"], "disk");
    }

    #[test]
    fn rejects_malformed_pairs_lines() {
        assert!(matches!(
            parse_pairs_line("NAME=sda TYPE=\"disk\""),
            Err(Error::LsblkOutput(_))
        ));
        assert!(matches!(
            parse_pairs_line("NAME=\"sda"),
            Err(Error::LsblkOutput(_))
        ));
    }

    #[test]
    fn empty_pairs_output_yields_no_targets() {
        assert!(parse_pairs("").unwrap().is_empty());
        assert!(parse_pairs("  \n \n").unwrap().is_empty());
    }
}
