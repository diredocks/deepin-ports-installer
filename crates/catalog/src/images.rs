use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::text::contains_ignore_case;
use crate::{Arch, DeviceIndex, Error, Result, date};

/// An image group that matched a [`Query`] but needs disambiguation.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Candidate {
    /// Architecture of the matching group.
    pub arch: Arch,

    /// Device/SoC slug of the matching group.
    pub device: String,
}

impl fmt::Display for Candidate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.arch, self.device)
    }
}

/// Criteria used to pick an image out of an [`ImageIndex`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query<'a> {
    /// Device model or image device slug, e.g. `visionfive-2` or `jh7110`.
    pub device: &'a str,

    /// Architecture filter, needed when a device exists on several of them.
    pub arch: Option<Arch>,

    /// Variant substring filter, e.g. `25-crimson`.
    pub variant: Option<&'a str>,

    /// Publication date (`YYYYMMDD`) to select; when absent the newest matching
    /// image wins.
    pub date: Option<&'a str>,
}

/// A single downloadable image entry, i.e. one `[[images]]` record.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Image {
    /// Architecture, e.g. `riscv`.
    pub arch: Arch,

    /// Device/SoC slug, e.g. `jh7110`.
    pub device: String,

    /// Release variant, e.g. `25-crimson-preview`.
    pub variant: String,

    pub size: u64,

    /// Build timestamp, e.g. `20250625-030117`.
    pub date: String,

    pub link: String,
}

/// Criteria used to pick images out of an [`ImageIndex`].
///
/// Every field is optional; an unset field matches everything. Text fields are
/// matched case-insensitively as substrings, like [`Query`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ImageQuery<'a> {
    /// Only match this architecture.
    pub arch: Option<Arch>,

    /// Only match images whose device slug contains this text.
    pub device: Option<&'a str>,

    /// Only match variants containing this text, e.g. `25-crimson`.
    pub variant: Option<&'a str>,

    /// Only match builds published on this day (`YYYYMMDD` or `YYYY-MM-DD`).
    pub date: Option<&'a str>,

    /// Keep only the newest matching image of each device.
    pub latest: bool,
}

impl ImageQuery<'_> {
    /// Whether `image` satisfies every set field.
    fn matches(&self, image: &Image) -> bool {
        self.arch.is_none_or(|arch| image.arch == arch)
            && self
                .device
                .is_none_or(|device| contains_ignore_case(&image.device, device))
            && self
                .variant
                .is_none_or(|variant| contains_ignore_case(&image.variant, variant))
            && self
                .date
                .is_none_or(|day| date::is_on_day(&image.date, day))
    }
}

/// The `images.toml` bundled into the binary at compile time.
const EMBEDDED_IMAGES: &str = include_str!("../assets/images.toml");

/// The top-level shape of `images.toml`.
#[derive(Debug, Deserialize)]
struct ImagesFile {
    #[serde(default)]
    images: Vec<Image>,
}

/// Every image published for the supported architectures.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImageIndex {
    images: Vec<Image>,
}

impl ImageIndex {
    /// Parse an `images.toml` document.
    ///
    /// The document is a flat list of `[[images]]` records; the index keeps
    /// them sorted by architecture, device and date, so the images of a device
    /// are adjacent and ordered chronologically.
    pub fn parse(document: &str) -> Result<Self> {
        let file: ImagesFile = toml::from_str(document)?;
        Ok(Self::from(file.images))
    }

    pub fn parse_file(path: impl AsRef<Path>) -> Result<Self> {
        Self::parse(&crate::read_document(path.as_ref())?)
    }

    pub fn embedded() -> Result<Self> {
        Self::parse(EMBEDDED_IMAGES)
    }

    pub fn images(&self) -> &[Image] {
        &self.images
    }

    /// Resolve the newest image matching `query`.
    ///
    /// The device must match an image's device slug exactly, or be a known
    /// device model that maps to that image's architecture and SoC. More than
    /// one match is an error; use `query.arch` (or a more specific device) to
    /// disambiguate.
    pub fn resolve(&self, devices: &DeviceIndex, query: &Query<'_>) -> Result<Image> {
        let (arch, device) = self.select_device(devices, query)?;

        self.images
            .iter()
            .rev()
            .find(|image| {
                image.arch == arch
                    && image.device == device
                    && query
                        .variant
                        .is_none_or(|variant| contains_ignore_case(&image.variant, variant))
                    && query
                        .date
                        .is_none_or(|day| date::is_on_day(&image.date, day))
            })
            .cloned()
            .ok_or_else(|| no_image(query))
    }

    /// The images matching `query`, newest last, with `query.latest` applied.
    pub fn filter(&self, query: &ImageQuery<'_>) -> Vec<Image> {
        let matched = || self.images.iter().filter(|image| query.matches(image));

        if !query.latest {
            return matched().cloned().collect();
        }

        // Images are sorted by date within a device, so the newest match of a
        // device is its last one.
        let mut newest: BTreeMap<(Arch, &str), &Image> = BTreeMap::new();
        for image in matched() {
            newest
                .entry((image.arch, image.device.as_str()))
                .and_modify(|current| {
                    if image.date > current.date {
                        *current = image;
                    }
                })
                .or_insert(image);
        }

        newest.into_values().cloned().collect()
    }

    /// An index holding only the images matching `query`.
    pub fn filtered(&self, query: &ImageQuery<'_>) -> Self {
        Self::from(self.filter(query))
    }

    /// The `(arch, device)` pair `query` refers to, or an error when no or
    /// several devices match.
    fn select_device<'a>(
        &'a self,
        devices: &DeviceIndex,
        query: &Query<'_>,
    ) -> Result<(Arch, &'a str)> {
        let mut groups: Vec<(Arch, &str)> = Vec::new();
        for image in &self.images {
            if query.arch.is_some_and(|arch| arch != image.arch) {
                continue;
            }

            let matches = image.device.eq_ignore_ascii_case(query.device)
                || devices.maps_to(query.device, image.arch, &image.device);
            if matches && groups.last() != Some(&(image.arch, image.device.as_str())) {
                groups.push((image.arch, image.device.as_str()));
            }
        }

        match groups.as_slice() {
            [] => Err(no_image(query)),
            [device] => Ok(*device),
            _ => Err(Error::Ambiguous {
                device: query.device.to_owned(),
                candidates: groups
                    .iter()
                    .map(|(arch, device)| Candidate {
                        arch: *arch,
                        device: (*device).to_owned(),
                    })
                    .collect(),
            }),
        }
    }

    pub fn to_json(&self) -> Result<String> {
        crate::to_json(&self.images)
    }

    /// Serialize the images back into the canonical flat `[[images]]` layout
    /// used by `images.toml`.
    pub fn to_toml(&self) -> Result<String> {
        #[derive(Serialize)]
        struct Document<'a> {
            images: &'a [Image],
        }

        Ok(toml::to_string_pretty(&Document {
            images: &self.images,
        })?)
    }
}

impl From<Vec<Image>> for ImageIndex {
    /// Images are sorted by architecture, device and date, so each device's
    /// images are adjacent and the newest image is always last.
    fn from(mut images: Vec<Image>) -> Self {
        images.sort_by(|a, b| (&a.arch, &a.device, &a.date).cmp(&(&b.arch, &b.device, &b.date)));
        Self { images }
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
        [[images]]
        arch = "riscv"
        device = "dp1000"
        variant = "25-crimson-preview"
        size = 3908305680
        date = "20250625-030117"
        link = "https://example.com/dp1000.tar.xz"

        [[images]]
        arch = "riscv"
        device = "jh7110"
        variant = "23-beige-preview"
        size = 3010718680
        date = "20240805-110418"
        link = "https://example.com/jh7110-1.tar.xz"

        [[images]]
        arch = "riscv"
        device = "jh7110"
        variant = "25-crimson-preview"
        size = 3652822392
        date = "20250603-113217"
        link = "https://example.com/jh7110-2.tar.xz"

        [[images]]
        arch = "arm64"
        device = "generic"
        variant = "25-crimson-immutable"
        size = 5521922048
        date = "20260805-185035"
        link = "https://example.com/arm64.iso"
    "#;

    #[test]
    fn parses_images_in_sorted_order() {
        let index = ImageIndex::parse(SAMPLE).unwrap();
        assert_eq!(index.images().len(), 4);

        let devices: Vec<(Arch, &str)> = index
            .images()
            .iter()
            .map(|image| (image.arch, image.device.as_str()))
            .collect();
        assert_eq!(
            devices,
            [
                (Arch::Riscv, "dp1000"),
                (Arch::Riscv, "jh7110"),
                (Arch::Riscv, "jh7110"),
                (Arch::Arm64, "generic"),
            ]
        );

        let jh7110 = &index.images()[1];
        assert_eq!(jh7110.variant, "23-beige-preview");
        assert_eq!(jh7110.size, 3010718680);
        assert_eq!(jh7110.date, "20240805-110418");
        assert_eq!(index.images()[2].variant, "25-crimson-preview");
    }

    #[test]
    fn sorts_images_by_device_then_date() {
        let index = ImageIndex::from(vec![
            image(
                Arch::Riscv,
                "k1",
                "25-crimson-preview",
                "20250101-000000",
                "https://e/new",
            ),
            image(
                Arch::Arm64,
                "k1",
                "25-crimson-preview",
                "20240101-000000",
                "https://e/arm",
            ),
            image(
                Arch::Riscv,
                "k1",
                "23-beige-preview",
                "20240101-000000",
                "https://e/old",
            ),
        ]);

        let links: Vec<&str> = index
            .images()
            .iter()
            .map(|image| image.link.as_str())
            .collect();
        assert_eq!(links, ["https://e/old", "https://e/new", "https://e/arm"]);
    }

    #[test]
    fn to_toml_round_trips_canonical_layout() {
        let index = ImageIndex::parse(SAMPLE).unwrap();
        let rendered = index.to_toml().unwrap();

        assert!(rendered.starts_with("[[images]]\narch = \"riscv\"\ndevice = "));
        assert_eq!(ImageIndex::parse(&rendered).unwrap(), index);
    }

    fn image(arch: Arch, device: &str, variant: &str, date: &str, link: &str) -> Image {
        Image {
            arch,
            device: device.to_owned(),
            variant: variant.to_owned(),
            size: 1,
            date: date.to_owned(),
            link: link.to_owned(),
        }
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
        let index = ImageIndex::from(vec![
            image(
                Arch::Riscv,
                "k1",
                "23-beige-preview",
                "20240101-000000",
                "https://e/old.tar.xz",
            ),
            image(
                Arch::Riscv,
                "k1",
                "25-crimson-preview",
                "20250101-000000",
                "https://e/new.tar.xz",
            ),
        ]);
        let devices = model(Arch::Riscv, "k1", &["muse-pi"]);

        let resolved = index.resolve(&devices, &query("muse-pi")).unwrap();
        assert_eq!(resolved.date, "20250101-000000");
    }

    #[test]
    fn maps_model_across_architectures() {
        let index = ImageIndex::from(vec![
            image(
                Arch::Riscv,
                "generic",
                "v",
                "20240101-000000",
                "https://e/rv",
            ),
            image(
                Arch::Arm64,
                "generic",
                "v",
                "20250101-000000",
                "https://e/arm",
            ),
        ]);
        let devices = model(Arch::Arm64, "generic", &["qemu-arm"]);

        let resolved = index.resolve(&devices, &query("qemu-arm")).unwrap();
        assert_eq!(resolved.link, "https://e/arm");
    }

    #[test]
    fn matches_device_slug_exactly() {
        let index = ImageIndex::from(vec![image(
            Arch::Riscv,
            "visionfive2",
            "v",
            "20240101-000000",
            "https://e/vf2",
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
        let index = ImageIndex::from(vec![
            image(
                Arch::Riscv,
                "generic",
                "v",
                "20240101-000000",
                "https://e/rv",
            ),
            image(
                Arch::Arm64,
                "generic",
                "v",
                "20250101-000000",
                "https://e/arm",
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
        let index = ImageIndex::from(vec![
            image(
                Arch::Riscv,
                "k1",
                "23-beige-preview",
                "20240101-000000",
                "https://e/beige",
            ),
            image(
                Arch::Riscv,
                "k1",
                "25-crimson-preview",
                "20250101-000000",
                "https://e/crimson",
            ),
        ]);

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
    fn filters_images_by_arch_device_and_latest() {
        let index = ImageIndex::parse(SAMPLE).unwrap();

        let all = index.filter(&ImageQuery::default());
        assert_eq!(all.len(), 4);

        let jh7110 = index.filter(&ImageQuery {
            arch: Some(Arch::Riscv),
            device: Some("JH"),
            ..ImageQuery::default()
        });
        assert_eq!(jh7110.len(), 2);

        let latest = index.filter(&ImageQuery {
            arch: Some(Arch::Riscv),
            device: Some("JH"),
            latest: true,
            ..ImageQuery::default()
        });
        assert_eq!(latest.len(), 1);
        assert_eq!(latest[0].device, "jh7110");
        assert_eq!(latest[0].variant, "25-crimson-preview");

        let beige = index.filter(&ImageQuery {
            variant: Some("BEIGE"),
            ..ImageQuery::default()
        });
        assert_eq!(beige.len(), 1);
        assert_eq!(beige[0].device, "jh7110");

        assert!(
            index
                .filter(&ImageQuery {
                    variant: Some("nonexistent"),
                    ..ImageQuery::default()
                })
                .is_empty()
        );
    }

    #[test]
    fn reports_missing_images() {
        let index = ImageIndex::from(vec![image(
            Arch::Riscv,
            "k1",
            "v",
            "20240101-000000",
            "https://e/a",
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
