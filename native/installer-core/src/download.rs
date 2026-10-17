//! Resolve and download Deepin Ports images.
//!
//! Resolution maps a user-facing device model (or an image-group slug) to a
//! single [`Image`]; downloading streams that image to a local path.

use std::path::{Path, PathBuf};

use url::Url;

use crate::text::contains_ignore_case;
use crate::{Arch, DeviceIndex, Error, Image, ImageGroup, ImageIndex, Result};

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

/// Resolve the newest image matching `query`.
///
/// The device must match an image-group slug exactly, or be a known device
/// model that maps to that group's `arch` and SoC. More than one match is an
/// error; use `arch` (or a more specific device) to disambiguate.
pub fn resolve(images: &ImageIndex, devices: &DeviceIndex, query: &Query<'_>) -> Result<Image> {
    let group = select_group(images, devices, query)?;
    select_image(group, query)
}

/// Pick the single group that `query.device` names, failing when it names none
/// or several.
fn select_group<'a>(
    images: &'a ImageIndex,
    devices: &DeviceIndex,
    query: &Query<'_>,
) -> Result<&'a ImageGroup> {
    let groups: Vec<&ImageGroup> = images
        .groups()
        .iter()
        .filter(|group| query.arch.is_none_or(|arch| group.arch == arch))
        .filter(|group| {
            group.device.eq_ignore_ascii_case(query.device) || maps_to(devices, query.device, group)
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

/// Pick the newest image in `group` that also matches the variant and date
/// filters. Group images are sorted by date, so the last match is the newest.
fn select_image(group: &ImageGroup, query: &Query<'_>) -> Result<Image> {
    let date = query.date.map(normalize);

    group
        .images
        .iter()
        .rfind(|image| {
            query
                .variant
                .is_none_or(|variant| contains_ignore_case(&image.variant, variant))
                && date
                    .as_ref()
                    .is_none_or(|date| normalize(&image.date).starts_with(date))
        })
        .cloned()
        .ok_or_else(|| no_image(query))
}

/// The "nothing matched" error for `query`.
fn no_image(query: &Query<'_>) -> Error {
    Error::NoImage {
        device: query.device.to_owned(),
        variant: query.variant.map(str::to_owned),
        date: query.date.map(str::to_owned),
    }
}

/// Resolve where `image` should be written.
///
/// A destination that is an existing directory, or that ends with a path
/// separator, has the image's file name appended. Anything else is used
/// verbatim. Without a destination, the image is placed in the `images`
/// subdirectory of the platform cache directory.
pub fn destination(output: Option<&Path>, image: &Image) -> Result<PathBuf> {
    let file_name =
        file_name(&image.link).ok_or_else(|| Error::LinkFileName(image.link.clone()))?;

    match output {
        None => Ok(crate::paths::cache_dir()?.join("images").join(file_name)),
        Some(path) if is_directory(path) => Ok(path.join(file_name)),
        Some(path) => Ok(path.to_path_buf()),
    }
}

/// Progress reported while downloading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    /// Bytes written so far.
    pub downloaded: u64,

    /// Total size in bytes, when known.
    pub total: Option<u64>,
}

/// Download `image` to `dest`, calling `progress` as data arrives.
///
/// The body is streamed to a temporary sibling of `dest` and moved into place
/// once complete, so `dest` never holds a partial image. Unless `force` is set,
/// an existing `dest` is refused.
#[cfg(feature = "http")]
pub async fn download<F>(image: &Image, dest: &Path, force: bool, mut progress: F) -> Result<()>
where
    F: FnMut(Progress),
{
    if !force && dest.exists() {
        return Err(Error::AlreadyExists {
            path: dest.to_path_buf(),
        });
    }

    if let Some(parent) = dest
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(|source| Error::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let response = crate::http::response(&image.link).await?;
    let total = response.content_length().or(Some(image.size));

    let temp = temp_path(dest);
    let mut file = std::fs::File::create(&temp).map_err(|source| Error::Write {
        path: temp.clone(),
        source,
    })?;

    if let Err(error) = write_body(
        &mut file,
        response,
        &temp,
        &image.link,
        total,
        &mut progress,
    )
    .await
    {
        drop(file);
        let _ = std::fs::remove_file(&temp);
        return Err(error);
    }

    if let Err(source) = std::io::Write::flush(&mut file) {
        drop(file);
        let _ = std::fs::remove_file(&temp);
        return Err(Error::Write { path: temp, source });
    }
    drop(file);

    if force && dest.exists() {
        std::fs::remove_file(dest).map_err(|source| Error::Write {
            path: dest.to_path_buf(),
            source,
        })?;
    }

    std::fs::rename(&temp, dest).map_err(|source| Error::Write {
        path: dest.to_path_buf(),
        source,
    })?;

    Ok(())
}

/// Stream a response body into `file`, reporting progress.
#[cfg(feature = "http")]
async fn write_body<F>(
    file: &mut std::fs::File,
    mut response: reqwest::Response,
    dest: &Path,
    url: &str,
    total: Option<u64>,
    progress: &mut F,
) -> Result<()>
where
    F: FnMut(Progress),
{
    use std::io::Write;

    let mut downloaded = 0;
    while let Some(chunk) = response.chunk().await.map_err(|source| Error::Request {
        url: url.to_owned(),
        source,
    })? {
        file.write_all(&chunk).map_err(|source| Error::Write {
            path: dest.to_path_buf(),
            source,
        })?;

        downloaded += chunk.len() as u64;
        progress(Progress { downloaded, total });
    }

    Ok(())
}

/// Whether a device model named `model` maps to `group` via its SoC.
fn maps_to(devices: &DeviceIndex, model: &str, group: &ImageGroup) -> bool {
    devices.groups().iter().any(|entry| {
        entry.arch == group.arch
            && entry.soc.eq_ignore_ascii_case(&group.device)
            && entry
                .devices
                .iter()
                .any(|device| device.eq_ignore_ascii_case(model))
    })
}

/// Digits-only form so dates compare regardless of separators (`2024-01-01` →
/// `20240101`).
fn normalize(value: &str) -> String {
    value
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|character| character.to_ascii_lowercase())
        .collect()
}

/// Whether `path` refers to a directory.
fn is_directory(path: &Path) -> bool {
    path.is_dir() || path.to_string_lossy().ends_with(['/', '\\'])
}

/// The final path segment of a link, ignoring any query or fragment.
fn file_name(link: &str) -> Option<String> {
    Url::parse(link)
        .ok()?
        .path_segments()?
        .next_back()
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
}

/// A temporary sibling of `dest` used while the download is in flight.
#[cfg(any(feature = "http", test))]
fn temp_path(dest: &Path) -> PathBuf {
    let mut name = std::ffi::OsString::from(".");
    name.push(dest.file_name().map_or_else(
        || std::ffi::OsString::from("download"),
        |name| name.to_os_string(),
    ));
    name.push(format!(".part-{}", std::process::id()));
    dest.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Arch, DeviceGroup};

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
        let images = images(vec![(
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

        let resolved = resolve(&images, &devices, &query("muse-pi")).unwrap();
        assert_eq!(resolved.date, "20250101-000000");
    }

    #[test]
    fn maps_model_across_architectures() {
        let images = images(vec![
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

        let resolved = resolve(&images, &devices, &query("qemu-arm")).unwrap();
        assert_eq!(resolved.link, "https://e/arm");
    }

    #[test]
    fn matches_group_slug_exactly() {
        let images = images(vec![(
            Arch::Riscv,
            "visionfive2",
            vec![image("v", "20240101-000000", "https://e/vf2")],
        )]);

        let resolved = resolve(&images, &DeviceIndex::default(), &query("VISIONFIVE2")).unwrap();
        assert_eq!(resolved.link, "https://e/vf2");

        assert!(matches!(
            resolve(&images, &DeviceIndex::default(), &query("visionfive-2")),
            Err(Error::NoImage { .. })
        ));
    }

    #[test]
    fn narrows_ambiguous_match_with_arch() {
        let images = images(vec![
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
            resolve(&images, &devices, &query("generic")),
            Err(Error::Ambiguous { candidates, .. }) if candidates.len() == 2
        ));

        let scoped = Query {
            device: "generic",
            arch: Some(Arch::Arm64),
            variant: None,
            date: None,
        };
        assert_eq!(
            resolve(&images, &devices, &scoped).unwrap().link,
            "https://e/arm"
        );
    }

    #[test]
    fn filters_by_variant_and_date() {
        let images = images(vec![(
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
            resolve(&images, &DeviceIndex::default(), &variant)
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
            resolve(&images, &DeviceIndex::default(), &dated)
                .unwrap()
                .link,
            "https://e/beige"
        );
    }

    #[test]
    fn reports_missing_images() {
        let images = images(vec![(
            Arch::Riscv,
            "k1",
            vec![image("v", "20240101-000000", "https://e/a")],
        )]);

        assert!(matches!(
            resolve(&images, &DeviceIndex::default(), &query("nope")),
            Err(Error::NoImage { .. })
        ));

        let missing_variant = Query {
            device: "k1",
            arch: None,
            variant: Some("nonexistent"),
            date: None,
        };
        assert!(matches!(
            resolve(&images, &DeviceIndex::default(), &missing_variant),
            Err(Error::NoImage { .. })
        ));
    }

    #[test]
    fn derives_file_name_from_link() {
        assert_eq!(
            file_name("https://e/a/b.tar.xz").as_deref(),
            Some("b.tar.xz")
        );
        assert_eq!(
            file_name("https://e/a/b.tar.xz?token=1#frag").as_deref(),
            Some("b.tar.xz")
        );
        assert_eq!(file_name("https://e/").as_deref(), None);
        assert_eq!(file_name("https://e").as_deref(), None);
        assert_eq!(file_name("not a url").as_deref(), None);
    }

    #[test]
    fn builds_destination_from_link_and_output() {
        let sample = image(
            "25-crimson-preview",
            "20250101-000000",
            "https://e/20250101/riscv64/deepin-25-crimson-preview-riscv64-k1-20250101-000000.tar.xz",
        );

        assert_eq!(
            destination(None, &sample).unwrap(),
            crate::paths::cache_dir()
                .unwrap()
                .join("images")
                .join("deepin-25-crimson-preview-riscv64-k1-20250101-000000.tar.xz")
        );

        assert_eq!(
            destination(Some(Path::new("/tmp/custom.img")), &sample).unwrap(),
            PathBuf::from("/tmp/custom.img")
        );

        assert_eq!(
            destination(Some(&std::env::temp_dir()), &sample).unwrap(),
            std::env::temp_dir()
                .join("deepin-25-crimson-preview-riscv64-k1-20250101-000000.tar.xz")
        );

        assert!(matches!(
            destination(None, &image("v", "d", "https://e/")),
            Err(Error::LinkFileName(_))
        ));
    }

    #[test]
    fn temporary_path_is_a_sibling() {
        let temp = temp_path(Path::new("/tmp/image.tar.xz"));
        assert_eq!(temp.parent(), Some(Path::new("/tmp")));
        assert!(
            temp.file_name()
                .unwrap()
                .to_string_lossy()
                .contains(".part-")
        );
    }
}
