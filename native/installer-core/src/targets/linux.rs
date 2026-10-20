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
//!
//! Both readers normalize their rows into [`Device`], so the derived flags and
//! the [`Target`] layout are written down exactly once.

mod json;
mod pairs;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{BusType, Enumerator, Mountpoint, PartitionTable, Target, TargetList};
use crate::{Error, Result};

/// The `lsblk` binary.
const LSBLK: &str = "lsblk";

/// Directory holding stable device links.
const DISK_PATH_DIR: &str = "/dev/disk/by-path";

/// Enumerate local block devices.
pub(super) fn scan() -> Result<TargetList> {
    let mut targets = match json::scan() {
        Ok(targets) => targets,
        Err(_) => pairs::scan()?,
    };

    add_device_paths(&mut targets);
    Ok(TargetList::new(targets))
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

// --- normalized device rows -------------------------------------------------

/// A backend independent `lsblk` device row.
///
/// The JSON and `--pairs` readers both fill this in; [`Device::into_target`]
/// then derives the flags that `drivelist` reports from the raw fields.
#[derive(Debug, Default)]
struct Device {
    /// Device node, e.g. `/dev/sdb`, already resolved and filtered.
    name: PathBuf,

    /// Kernel device path; defaults to `name` when the backend reports none.
    raw: Option<PathBuf>,

    label: Option<String>,
    vendor: Option<String>,
    model: Option<String>,
    size: Option<u64>,
    block_size: Option<u64>,
    logical_block_size: Option<u64>,
    mountpoints: Vec<Mountpoint>,
    transport: Option<BusType>,
    is_virtual: bool,
    is_read_only: bool,
    is_removable: bool,
    partition_table: Option<PartitionTable>,
}

impl Device {
    /// Start a row from an `lsblk` name, dropping loop/optical/RAM devices.
    fn new(name: &str) -> Option<Self> {
        let name = resolve_device(name);

        (!is_ignored(&name)).then(|| Self {
            raw: Some(name.clone()),
            name,
            ..Self::default()
        })
    }

    /// Record the flags `lsblk` reports; virtual devices always count as
    /// removable, matching `drivelist`.
    fn set_flags(&mut self, read_only: bool, removable: bool, hotplug: bool) {
        self.is_read_only = read_only;
        self.is_removable = removable || hotplug || self.is_virtual;
    }

    /// Fill in the transport, which yields [`Target::bus_type`].
    ///
    /// Normalization (trimming, case, empty values) is [`BusType::from`]'s job.
    fn set_transport(&mut self, transport: Option<&str>) {
        self.transport = transport.map(BusType::from);
    }

    /// Convert the row into a [`Target`].
    fn into_target(self, enumerator: Enumerator) -> Target {
        let transport = self.transport.as_ref();

        Target {
            enumerator,
            bus_type: transport.cloned().unwrap_or_else(BusType::unreported),
            bus_version: None,
            description: describe(
                self.label.as_deref(),
                self.vendor.as_deref(),
                self.model.as_deref(),
                &self.name.display().to_string(),
            ),
            raw: self.raw.unwrap_or_else(|| self.name.clone()),
            device: self.name,
            device_path: None,
            size: self.size,
            block_size: self.block_size.unwrap_or(512),
            logical_block_size: self.logical_block_size.unwrap_or(512),
            mountpoints: self.mountpoints,
            is_read_only: self.is_read_only,
            is_system: !self.is_removable && !self.is_virtual,
            is_virtual: self.is_virtual,
            is_removable: self.is_removable,
            is_card: None,
            is_uas: None,
            partition_table: self.partition_table,
        }
    }
}

// --- shared helpers ---------------------------------------------------------

/// Whether `lsblk`'s subsystem list marks the device as virtual.
///
/// `drivelist` treats devices whose only subsystem is `block` (device mapper,
/// LVM, ...) as virtual.
fn is_virtual(subsystems: Option<&str>) -> bool {
    subsystems.is_some_and(|subsystems| subsystems.eq_ignore_ascii_case("block"))
}

/// A non-empty view of an optional string field.
fn text(value: &Option<String>) -> Option<&str> {
    value.as_deref().filter(|value| !value.is_empty())
}

/// Whether a textual flag (`"0"`, `"1"`, `"true"`, ...) is set.
fn text_is_true(value: &str) -> bool {
    !matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "" | "0" | "false" | "no" | "off"
    )
}

/// Join labels, vendor and model into a single description.
fn describe(
    label: Option<&str>,
    vendor: Option<&str>,
    model: Option<&str>,
    fallback: &str,
) -> String {
    let parts: Vec<&str> = [label, vendor, model]
        .into_iter()
        .flatten()
        .flat_map(str::split_whitespace)
        .collect();

    if parts.is_empty() {
        fallback.to_owned()
    } else {
        parts.join(" ")
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
    ["/dev/loop", "/dev/sr", "/dev/ram"]
        .iter()
        .any(|prefix| device.starts_with(prefix))
}

/// Map an `lsblk` partition-table type to [`PartitionTable`].
fn partition_table(pttype: Option<&str>) -> Option<PartitionTable> {
    match pttype?.trim().to_ascii_lowercase().as_str() {
        "gpt" => Some(PartitionTable::Gpt),
        "dos" => Some(PartitionTable::Mbr),
        _ => None,
    }
}

/// Attach `/dev/disk/by-path` links to the matching targets.
fn add_device_paths(targets: &mut [Target]) {
    let Ok(entries) = std::fs::read_dir(DISK_PATH_DIR) else {
        return;
    };

    let links: HashMap<PathBuf, PathBuf> = entries
        .flatten()
        .filter_map(|entry| {
            let link = entry.path();
            Some((std::fs::canonicalize(&link).ok()?, link))
        })
        .collect();

    for target in targets {
        target.device_path = links.get(&target.device).cloned();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn describes_devices_from_labels_vendor_and_model() {
        assert_eq!(
            describe(
                Some("DEEPIN"),
                Some("SanDisk"),
                Some("Ultra  USB"),
                "/dev/sdb"
            ),
            "DEEPIN SanDisk Ultra USB"
        );
        assert_eq!(describe(None, None, Some("  "), "/dev/sdb"), "/dev/sdb");
        assert_eq!(describe(None, Some(""), None, "/dev/sda"), "/dev/sda");
    }

    #[test]
    fn marks_single_subsystem_devices_virtual() {
        assert!(is_virtual(Some("block")));
        assert!(is_virtual(Some("BLOCK")));
        assert!(!is_virtual(Some("block:scsi:usb:pci")));
        assert!(!is_virtual(None));
    }
}
