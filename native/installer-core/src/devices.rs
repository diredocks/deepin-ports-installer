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

/// The top-level shape of `devices.toml`.
#[derive(Debug, Deserialize)]
struct DevicesFile {
    #[serde(default)]
    devices: Vec<DeviceGroup>,
}

/// Parsed contents of `devices.toml`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceIndex {
    groups: Vec<DeviceGroup>,
}

impl DeviceIndex {
    /// Parse a `devices.toml` document.
    pub fn parse(document: &str) -> Result<Self> {
        let file: DevicesFile = toml::from_str(document)?;
        Ok(Self {
            groups: file.devices,
        })
    }

    /// Read and parse a `devices.toml` file.
    pub fn parse_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let document = std::fs::read_to_string(path).map_err(|source| Error::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&document)
    }

    /// All device groups, in file order.
    pub fn groups(&self) -> &[DeviceGroup] {
        &self.groups
    }

    /// Iterate over all device groups.
    pub fn iter(&self) -> std::slice::Iter<'_, DeviceGroup> {
        self.groups.iter()
    }

    /// Number of device groups.
    pub fn len(&self) -> usize {
        self.groups.len()
    }

    /// Whether the index contains no device groups.
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
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
    fn empty_document_yields_empty_index() {
        let index = DeviceIndex::parse("").unwrap();
        assert!(index.is_empty());
    }

    #[test]
    fn rejects_invalid_toml() {
        assert!(matches!(
            DeviceIndex::parse("[[devices]"),
            Err(Error::Parse(_))
        ));
    }
}
