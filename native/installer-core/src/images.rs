use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{Arch, Error, Result};

/// A single downloadable image entry, e.g. one `[[<arch>.<device>]]` table.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Image {
    /// Release variant, e.g. `25-crimson-preview`.
    pub variant: String,

    /// Size of the image in bytes.
    pub size: u64,

    /// Build timestamp, e.g. `20250625-030117`.
    pub date: String,

    /// Download URL.
    pub link: String,
}

/// All images available for one device under a specific architecture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImageGroup {
    /// Architecture, e.g. `riscv`.
    pub arch: Arch,

    /// Device/SoC slug, e.g. `jh7110`.
    pub device: String,

    /// Images for this device, sorted by `date` ascending.
    pub images: Vec<Image>,
}

/// The `images.toml` bundled into the binary at compile time.
const EMBEDDED_IMAGES: &str = include_str!("../assets/images.toml");

/// Parsed contents of `images.toml`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImageIndex {
    groups: Vec<ImageGroup>,
}

impl ImageIndex {
    /// Parse an `images.toml` document.
    ///
    /// The document is a two-level map, `[[<arch>.<device>]]`, that cannot be
    /// expressed as a plain struct. It is decoded into nested maps and then
    /// flattened into groups; the resulting order is sorted (by architecture,
    /// then device) rather than the order the groups appear in the file.
    pub fn parse(document: &str) -> Result<Self> {
        let parsed: BTreeMap<Arch, BTreeMap<String, Vec<Image>>> = toml::from_str(document)?;

        let groups: Vec<ImageGroup> = parsed
            .into_iter()
            .flat_map(|(arch, devices)| {
                devices.into_iter().map(move |(device, images)| ImageGroup {
                    arch,
                    device,
                    images,
                })
            })
            .collect();

        Ok(Self::from(groups))
    }

    /// Read and parse an `images.toml` file.
    pub fn parse_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let document = std::fs::read_to_string(path).map_err(|source| Error::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&document)
    }

    /// Parse the `images.toml` bundled into the binary at compile time.
    pub fn embedded() -> Result<Self> {
        Self::parse(EMBEDDED_IMAGES)
    }

    /// All image groups, in file order.
    pub fn groups(&self) -> &[ImageGroup] {
        &self.groups
    }

    /// Iterate over all image groups.
    pub fn iter(&self) -> std::slice::Iter<'_, ImageGroup> {
        self.groups.iter()
    }

    /// Number of image groups.
    pub fn len(&self) -> usize {
        self.groups.len()
    }

    /// Whether the index contains no image groups.
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    /// Serialize the image groups as a pretty-printed JSON array.
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(&self.groups)?)
    }

    /// Serialize the image groups back into the canonical
    /// `[[<arch>.<device>]]` layout used by `images.toml`.
    pub fn to_toml(&self) -> Result<String> {
        let mut root = toml::value::Table::new();

        for group in &self.groups {
            let arch = root
                .entry(group.arch.as_str().to_owned())
                .or_insert_with(|| toml::Value::Table(toml::value::Table::new()));

            let arch_table = arch
                .as_table_mut()
                .expect("architecture entry is always a table");

            let images: Vec<toml::Value> = group
                .images
                .iter()
                .map(toml::Value::try_from)
                .collect::<std::result::Result<_, toml::ser::Error>>()?;

            arch_table.insert(group.device.clone(), toml::Value::Array(images));
        }

        Ok(toml::to_string_pretty(&root)?)
    }
}

impl From<Vec<ImageGroup>> for ImageIndex {
    /// Groups are kept as given; each group's images are sorted by `date` so
    /// the newest image is always last.
    fn from(mut groups: Vec<ImageGroup>) -> Self {
        for group in &mut groups {
            group.images.sort_by(|a, b| a.date.cmp(&b.date));
        }
        Self { groups }
    }
}

impl<'a> IntoIterator for &'a ImageIndex {
    type Item = &'a ImageGroup;
    type IntoIter = std::slice::Iter<'a, ImageGroup>;

    fn into_iter(self) -> Self::IntoIter {
        self.groups.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
        [[riscv.dp1000]]
        variant = "25-crimson-preview"
        size = 3908305680
        date = "20250625-030117"
        link = "https://example.com/dp1000.tar.xz"

        [[riscv.jh7110]]
        variant = "23-beige-preview"
        size = 3010718680
        date = "20240805-110418"
        link = "https://example.com/jh7110-1.tar.xz"

        [[riscv.jh7110]]
        variant = "25-crimson-preview"
        size = 3652822392
        date = "20250603-113217"
        link = "https://example.com/jh7110-2.tar.xz"

        [[arm64.generic]]
        variant = "25-crimson-immutable"
        size = 5521922048
        date = "20260805-185035"
        link = "https://example.com/arm64.iso"
    "#;

    #[test]
    fn parses_groups_in_sorted_order() {
        let index = ImageIndex::parse(SAMPLE).unwrap();
        assert_eq!(index.len(), 3);

        let groups: Vec<(&str, &str)> = index
            .groups()
            .iter()
            .map(|g| (g.arch.as_str(), g.device.as_str()))
            .collect();
        assert_eq!(
            groups,
            [
                ("riscv", "dp1000"),
                ("riscv", "jh7110"),
                ("arm64", "generic")
            ]
        );

        let jh7110 = &index.groups()[1];
        assert_eq!(jh7110.images.len(), 2);
        assert_eq!(jh7110.images[0].variant, "23-beige-preview");
        assert_eq!(jh7110.images[0].size, 3010718680);
        assert_eq!(jh7110.images[0].date, "20240805-110418");
        assert_eq!(jh7110.images[1].variant, "25-crimson-preview");
    }

    #[test]
    fn images_are_sorted_by_date() {
        let index = ImageIndex::from(vec![ImageGroup {
            arch: Arch::Riscv,
            device: "jh7110".to_owned(),
            images: vec![
                Image {
                    variant: "new".to_owned(),
                    size: 2,
                    date: "20250603-113217".to_owned(),
                    link: "https://example.com/new.tar.xz".to_owned(),
                },
                Image {
                    variant: "old".to_owned(),
                    size: 1,
                    date: "20240805-110418".to_owned(),
                    link: "https://example.com/old.tar.xz".to_owned(),
                },
            ],
        }]);

        let variants: Vec<&str> = index.groups()[0]
            .images
            .iter()
            .map(|image| image.variant.as_str())
            .collect();
        assert_eq!(variants, ["old", "new"]);
    }

    #[test]
    fn empty_document_yields_empty_index() {
        let index = ImageIndex::parse("").unwrap();
        assert!(index.is_empty());
    }

    #[test]
    fn rejects_invalid_toml() {
        assert!(matches!(
            ImageIndex::parse("[[riscv.jh7110]"),
            Err(Error::Parse(_))
        ));
    }

    #[test]
    fn to_toml_round_trips_canonical_layout() {
        let index = ImageIndex::parse(SAMPLE).unwrap();
        let rendered = index.to_toml().unwrap();

        assert!(rendered.contains("[[riscv.dp1000]]"));
        assert!(rendered.contains("[[riscv.jh7110]]"));
        assert!(rendered.contains("[[arm64.generic]]"));

        assert_eq!(ImageIndex::parse(&rendered).unwrap(), index);
    }

    #[test]
    fn to_json_emits_group_array() {
        let index = ImageIndex::parse(SAMPLE).unwrap();
        let value: serde_json::Value = serde_json::from_str(&index.to_json().unwrap()).unwrap();

        let array = value.as_array().unwrap();
        assert_eq!(array.len(), 3);
        assert_eq!(array[0]["arch"], "riscv");
        assert_eq!(array[0]["device"], "dp1000");
        assert_eq!(array[1]["device"], "jh7110");
        assert_eq!(array[1]["images"].as_array().unwrap().len(), 2);
    }
}
