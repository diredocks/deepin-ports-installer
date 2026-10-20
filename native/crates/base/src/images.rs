use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::text::contains_ignore_case;
use crate::{Arch, DeviceIndex, Error, Result, date};

/// Criteria used to pick an image out of an [`ImageIndex`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query<'a> {
    /// Device model or image-group slug, e.g. `visionfive-2` or `jh7110`.
    pub device: &'a str,

    /// Architecture filter, needed when a device exists on several of them.
    pub arch: Option<Arch>,

    /// Variant substring filter, e.g. `25-crimson`.
    pub variant: Option<&'a str>,

    /// Publication date (`YYYYMMDD`) to select; when absent the newest matching
    /// image wins.
    pub date: Option<&'a str>,
}

/// A single downloadable image entry, e.g. one `[[<arch>.<device>]]` table.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Image {
    /// Release variant, e.g. `25-crimson-preview`.
    pub variant: String,

    pub size: u64,

    /// Build timestamp, e.g. `20250625-030117`.
    pub date: String,

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

impl ImageGroup {
    /// Images are sorted by date, so the last match is the newest.
    fn select_image(&self, query: &Query<'_>) -> Result<Image> {
        self.images
            .iter()
            .rfind(|image| {
                query
                    .variant
                    .is_none_or(|variant| contains_ignore_case(&image.variant, variant))
                    && query
                        .date
                        .is_none_or(|day| date::is_on_day(&image.date, day))
            })
            .cloned()
            .ok_or_else(|| no_image(query))
    }
}

/// The `images.toml` bundled into the binary at compile time.
const EMBEDDED_IMAGES: &str = include_str!("../assets/images.toml");

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

    pub fn parse_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let document = std::fs::read_to_string(path).map_err(|source| Error::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&document)
    }

    pub fn embedded() -> Result<Self> {
        Self::parse(EMBEDDED_IMAGES)
    }

    pub fn groups(&self) -> &[ImageGroup] {
        &self.groups
    }

    pub fn iter(&self) -> std::slice::Iter<'_, ImageGroup> {
        self.groups.iter()
    }

    pub fn len(&self) -> usize {
        self.groups.len()
    }

    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    /// Resolve the newest image matching `query`.
    ///
    /// The device must match an image-group slug exactly, or be a known device
    /// model that maps to that group's architecture and SoC. More than one
    /// match is an error; use `query.arch` (or a more specific device) to
    /// disambiguate.
    pub fn resolve(&self, devices: &DeviceIndex, query: &Query<'_>) -> Result<Image> {
        self.select_group(devices, query)?.select_image(query)
    }

    fn select_group<'a>(
        &'a self,
        devices: &DeviceIndex,
        query: &Query<'_>,
    ) -> Result<&'a ImageGroup> {
        let groups: Vec<&ImageGroup> = self
            .groups
            .iter()
            .filter(|group| query.arch.is_none_or(|arch| group.arch == arch))
            .filter(|group| {
                group.device.eq_ignore_ascii_case(query.device)
                    || devices.maps_to(query.device, group.arch, &group.device)
            })
            .collect();

        match groups.as_slice() {
            [] => Err(no_image(query)),
            [group] => Ok(group),
            _ => Err(Error::Ambiguous {
                device: query.device.to_owned(),
                candidates: groups
                    .iter()
                    .map(|group| format!("{}/{}", group.arch, group.device))
                    .collect(),
            }),
        }
    }

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

fn no_image(query: &Query<'_>) -> Error {
    Error::NoImage {
        device: query.device.to_owned(),
        variant: query.variant.map(str::to_owned),
        date: query.date.map(str::to_owned),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DeviceGroup;

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
    fn to_toml_round_trips_canonical_layout() {
        let index = ImageIndex::parse(SAMPLE).unwrap();
        let rendered = index.to_toml().unwrap();

        assert!(rendered.contains("[[riscv.dp1000]]"));
        assert!(rendered.contains("[[riscv.jh7110]]"));
        assert!(rendered.contains("[[arm64.generic]]"));

        assert_eq!(ImageIndex::parse(&rendered).unwrap(), index);
    }

    fn image(variant: &str, date: &str, link: &str) -> Image {
        Image {
            variant: variant.to_owned(),
            size: 1,
            date: date.to_owned(),
            link: link.to_owned(),
        }
    }

    fn images(groups: Vec<(Arch, &str, Vec<Image>)>) -> ImageIndex {
        ImageIndex::from(
            groups
                .into_iter()
                .map(|(arch, device, images)| ImageGroup {
                    arch,
                    device: device.to_owned(),
                    images,
                })
                .collect::<Vec<_>>(),
        )
    }

    fn model(arch: Arch, soc: &str, devices: &[&str]) -> DeviceIndex {
        DeviceIndex::from(vec![DeviceGroup {
            arch,
            platform: "generic".to_owned(),
            soc: soc.to_owned(),
            devices: devices.iter().map(|device| (*device).to_owned()).collect(),
            links: Vec::new(),
        }])
    }

    fn query<'a>(device: &'a str) -> Query<'a> {
        Query {
            device,
            arch: None,
            variant: None,
            date: None,
        }
    }

    #[test]
    fn resolves_device_model_through_soc() {
        let index = images(vec![(
            Arch::Riscv,
            "k1",
            vec![
                image(
                    "23-beige-preview",
                    "20240101-000000",
                    "https://e/old.tar.xz",
                ),
                image(
                    "25-crimson-preview",
                    "20250101-000000",
                    "https://e/new.tar.xz",
                ),
            ],
        )]);
        let devices = model(Arch::Riscv, "k1", &["muse-pi"]);

        let resolved = index.resolve(&devices, &query("muse-pi")).unwrap();
        assert_eq!(resolved.date, "20250101-000000");
    }

    #[test]
    fn maps_model_across_architectures() {
        let index = images(vec![
            (
                Arch::Riscv,
                "generic",
                vec![image("v", "20240101-000000", "https://e/rv")],
            ),
            (
                Arch::Arm64,
                "generic",
                vec![image("v", "20250101-000000", "https://e/arm")],
            ),
        ]);
        let devices = model(Arch::Arm64, "generic", &["qemu-arm"]);

        let resolved = index.resolve(&devices, &query("qemu-arm")).unwrap();
        assert_eq!(resolved.link, "https://e/arm");
    }

    #[test]
    fn matches_group_slug_exactly() {
        let index = images(vec![(
            Arch::Riscv,
            "visionfive2",
            vec![image("v", "20240101-000000", "https://e/vf2")],
        )]);

        let resolved = index
            .resolve(&DeviceIndex::default(), &query("VISIONFIVE2"))
            .unwrap();
        assert_eq!(resolved.link, "https://e/vf2");

        assert!(matches!(
            index.resolve(&DeviceIndex::default(), &query("visionfive-2")),
            Err(Error::NoImage { .. })
        ));
    }

    #[test]
    fn narrows_ambiguous_match_with_arch() {
        let index = images(vec![
            (
                Arch::Riscv,
                "generic",
                vec![image("v", "20240101-000000", "https://e/rv")],
            ),
            (
                Arch::Arm64,
                "generic",
                vec![image("v", "20250101-000000", "https://e/arm")],
            ),
        ]);
        let devices = DeviceIndex::default();

        assert!(matches!(
            index.resolve(&devices, &query("generic")),
            Err(Error::Ambiguous { candidates, .. }) if candidates.len() == 2
        ));

        let scoped = Query {
            device: "generic",
            arch: Some(Arch::Arm64),
            variant: None,
            date: None,
        };
        assert_eq!(
            index.resolve(&devices, &scoped).unwrap().link,
            "https://e/arm"
        );
    }

    #[test]
    fn filters_by_variant_and_date() {
        let index = images(vec![(
            Arch::Riscv,
            "k1",
            vec![
                image("23-beige-preview", "20240101-000000", "https://e/beige"),
                image("25-crimson-preview", "20250101-000000", "https://e/crimson"),
            ],
        )]);

        let variant = Query {
            device: "k1",
            arch: None,
            variant: Some("beige"),
            date: None,
        };
        assert_eq!(
            index
                .resolve(&DeviceIndex::default(), &variant)
                .unwrap()
                .link,
            "https://e/beige"
        );

        let dated = Query {
            device: "k1",
            arch: None,
            variant: None,
            date: Some("2024-01-01"),
        };
        assert_eq!(
            index.resolve(&DeviceIndex::default(), &dated).unwrap().link,
            "https://e/beige"
        );
    }

    #[test]
    fn reports_missing_images() {
        let index = images(vec![(
            Arch::Riscv,
            "k1",
            vec![image("v", "20240101-000000", "https://e/a")],
        )]);

        assert!(matches!(
            index.resolve(&DeviceIndex::default(), &query("nope")),
            Err(Error::NoImage { .. })
        ));

        let missing_variant = Query {
            device: "k1",
            arch: None,
            variant: Some("nonexistent"),
            date: None,
        };
        assert!(matches!(
            index.resolve(&DeviceIndex::default(), &missing_variant),
            Err(Error::NoImage { .. })
        ));
    }
}
