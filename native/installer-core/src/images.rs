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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageGroup {
    /// Architecture, e.g. `riscv`.
    pub arch: Arch,

    /// Device/SoC slug, e.g. `jh7110`.
    pub device: String,

    /// Images for this device, in file order.
    pub images: Vec<Image>,
}

/// Parsed contents of `images.toml`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImageIndex {
    groups: Vec<ImageGroup>,
}

impl ImageIndex {
    /// Parse an `images.toml` document.
    ///
    /// The document is a two-level map, `[[<arch>.<device>]]`, that cannot be
    /// expressed as a plain struct, so it is walked manually to preserve the
    /// order in which the groups appear in the file.
    pub fn parse(document: &str) -> Result<Self> {
        let root: toml::Table = toml::from_str(document)?;
        let mut groups = Vec::new();

        for (arch, value) in &root {
            let arch: Arch = arch.parse()?;

            let arch_table = value.as_table().ok_or_else(|| {
                Error::Shape(format!(
                    "`{}` must be a table of device groups",
                    arch.as_str()
                ))
            })?;

            for (device, value) in arch_table {
                let images: Vec<Image> = value
                    .clone()
                    .try_into()
                    .map_err(|err| Error::Shape(format!("`{arch}.{device}`: {err}")))?;

                groups.push(ImageGroup {
                    arch,
                    device: device.clone(),
                    images,
                });
            }
        }

        Ok(Self { groups })
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
    fn parses_groups_in_file_order() {
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
}
