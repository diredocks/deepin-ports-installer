use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{Arch, Error, Result};

/// One `[[devices]]` entry from `devices.toml`: a set of device models that
/// share the same architecture, platform and SoC.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct DeviceGroup {
    /// Architecture, e.g. `riscv` or `arm64`.
    pub arch: Arch,

    /// CPU platform, e.g. `sifive-u74`.
    pub platform: String,

    /// SoC identifier, e.g. `jh7110`.
    pub soc: String,

    /// Device model slugs covered by this entry.
    pub devices: Vec<String>,

    /// Documentation links for installing on these devices.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<String>,
}

impl DeviceGroup {
    pub fn covers(&self, model: &str) -> bool {
        self.devices
            .iter()
            .any(|device| device.eq_ignore_ascii_case(model))
    }
}

/// The top-level shape of `devices.toml`.
#[derive(Debug, Deserialize)]
struct DevicesFile {
    #[serde(default)]
    devices: Vec<DeviceGroup>,
}

/// The `devices.toml` bundled into the binary at compile time.
const EMBEDDED_DEVICES: &str = include_str!("../assets/devices.toml");

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceIndex {
    groups: Vec<DeviceGroup>,
}

impl DeviceIndex {
    pub fn parse(document: &str) -> Result<Self> {
        let file: DevicesFile = toml::from_str(document)?;
        Ok(Self {
            groups: file.devices,
        })
    }

    pub fn parse_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let document = std::fs::read_to_string(path).map_err(|source| Error::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&document)
    }

    pub fn embedded() -> Result<Self> {
        Self::parse(EMBEDDED_DEVICES)
    }

    pub fn groups(&self) -> &[DeviceGroup] {
        &self.groups
    }

    pub fn iter(&self) -> std::slice::Iter<'_, DeviceGroup> {
        self.groups.iter()
    }

    pub fn len(&self) -> usize {
        self.groups.len()
    }

    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    /// Whether `model` is a documented device of the `soc` on `arch`.
    ///
    /// This bridges a user-facing device model (e.g. `visionfive-2`) to an
    /// image-group slug (e.g. `jh7110`) published as the SoC.
    pub fn maps_to(&self, model: &str, arch: Arch, soc: &str) -> bool {
        self.groups.iter().any(|group| {
            group.arch == arch && group.soc.eq_ignore_ascii_case(soc) && group.covers(model)
        })
    }

    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(&self.groups)?)
    }

    pub fn to_toml(&self) -> Result<String> {
        #[derive(Serialize)]
        struct Document<'a> {
            devices: &'a [DeviceGroup],
        }

        Ok(toml::to_string_pretty(&Document {
            devices: &self.groups,
        })?)
    }
}

impl From<Vec<DeviceGroup>> for DeviceIndex {
    fn from(groups: Vec<DeviceGroup>) -> Self {
        Self { groups }
    }
}

impl<'a> IntoIterator for &'a DeviceIndex {
    type Item = &'a DeviceGroup;
    type IntoIter = std::slice::Iter<'a, DeviceGroup>;

    fn into_iter(self) -> Self::IntoIter {
        self.groups.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
        [[devices]]
        arch = "riscv"
        platform = "sifive-u74"
        soc = "jh7110"
        devices = [
            "visionfive-2",
            "milkv-mars",
        ]
        links = [
            "https://example.com/jh7110",
        ]

        [[devices]]
        arch = "arm64"
        platform = "phytium"
        soc = "phytiumpi"
        devices = ["phytium-pi"]
    "#;

    #[test]
    fn parses_groups_in_file_order() {
        let index = DeviceIndex::parse(SAMPLE).unwrap();
        assert_eq!(index.len(), 2);

        let first = &index.groups()[0];
        assert_eq!(first.arch, Arch::Riscv);
        assert_eq!(first.platform, "sifive-u74");
        assert_eq!(first.soc, "jh7110");
        assert_eq!(first.devices, ["visionfive-2", "milkv-mars"]);
        assert_eq!(first.links, ["https://example.com/jh7110"]);

        let second = &index.groups()[1];
        assert_eq!(second.arch, Arch::Arm64);
        assert_eq!(second.devices, ["phytium-pi"]);
        assert!(second.links.is_empty());
    }

    #[test]
    fn to_toml_round_trips_devices_layout() {
        let index = DeviceIndex::parse(SAMPLE).unwrap();
        let rendered = index.to_toml().unwrap();

        assert!(rendered.contains("[[devices]]"));
        assert_eq!(DeviceIndex::parse(&rendered).unwrap(), index);
    }

    #[test]
    fn groups_cover_models_case_insensitively() {
        let index = DeviceIndex::parse(SAMPLE).unwrap();
        let group = &index.groups()[0];

        assert!(group.covers("VisionFive-2"));
        assert!(!group.covers("phytium-pi"));
    }

    #[test]
    fn maps_models_through_arch_and_soc() {
        let index = DeviceIndex::parse(SAMPLE).unwrap();

        assert!(index.maps_to("MILKV-MARS", Arch::Riscv, "JH7110"));
        assert!(!index.maps_to("milkv-mars", Arch::Arm64, "jh7110"));
        assert!(!index.maps_to("milkv-mars", Arch::Riscv, "phytiumpi"));
        assert!(!index.maps_to("unknown", Arch::Riscv, "jh7110"));
    }
}
