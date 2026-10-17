//! Resolve and download Deepin Ports images.
//!
//! Resolution maps a user-facing device model (or an image-group slug) to a
//! single [`Image`]; downloading streams that image to a local path.

use std::path::{Path, PathBuf};

use url::Url;

use crate::date;
use crate::paths;
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
    group
        .images
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
        None => Ok(paths::cache_dir()?.join("images").join(file_name)),
        Some(path) if paths::is_directory(path) => Ok(path.join(file_name)),
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
/// once complete, so `dest` never holds a partial image. An interrupted
/// download leaves that sibling behind; a later call resumes it with an HTTP
/// `Range` request instead of starting over. Unless `force` is set, an existing
/// `dest` is refused.
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

    let temp = temp_path(dest);
    let expected = (image.size > 0).then_some(image.size);

    // Resume wherever a previous run stopped, unless the leftover is already
    // as large as the image we expect (a stale partial from another build).
    let mut offset = partial_size(&temp);
    if expected.is_some_and(|total| offset >= total) {
        let _ = std::fs::remove_file(&temp);
        offset = 0;
    }

    let response = crate::http::response_from(&image.link, offset).await?;
    let (response, offset) = reconcile(response, &image.link, &temp, offset).await?;

    let total = response
        .content_length()
        .map(|length| length + offset)
        .or(expected);

    let mut file = open_partial(&temp, offset)?;
    progress(Progress {
        downloaded: offset,
        total,
    });

    let written = match write_body(
        &mut file,
        response,
        &temp,
        &image.link,
        offset,
        total,
        &mut progress,
    )
    .await
    {
        Ok(written) => written,
        Err(error) => {
            let _ = std::io::Write::flush(&mut file);
            return Err(error);
        }
    };

    if let Err(source) = std::io::Write::flush(&mut file) {
        drop(file);
        return Err(Error::Write { path: temp, source });
    }
    drop(file);

    if let Some(expected) = expected
        && written < expected
    {
        return Err(Error::Incomplete {
            url: image.link.clone(),
            downloaded: written,
            expected,
        });
    }

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

/// Reconcile a possibly-ranged response with the offset we asked to resume
/// from, returning the response to stream and the offset its body starts at.
///
/// A server that ignores the range, or reports it unsatisfiable, is handled by
/// restarting from the beginning so the resulting file is always contiguous.
#[cfg(feature = "http")]
async fn reconcile(
    response: reqwest::Response,
    url: &str,
    temp: &Path,
    offset: u64,
) -> Result<(reqwest::Response, u64)> {
    use reqwest::StatusCode;

    if offset == 0 {
        return ensure_success(response, url).map(|response| (response, 0));
    }

    match response.status() {
        // A genuine resume: the body starts exactly where the part file ends.
        StatusCode::PARTIAL_CONTENT if content_range_start(&response) == Some(offset) => {
            Ok((response, offset))
        }
        // A partial response we cannot place, or a range the server refuses:
        // drop the part file and fetch the whole image again.
        StatusCode::PARTIAL_CONTENT | StatusCode::RANGE_NOT_SATISFIABLE => {
            let _ = std::fs::remove_file(temp);
            let response = crate::http::response_from(url, 0).await?;
            ensure_success(response, url).map(|response| (response, 0))
        }
        // The server ignored the range and sent the whole image; reuse its body.
        status if status.is_success() => Ok((response, 0)),
        status => Err(Error::HttpStatus {
            url: url.to_owned(),
            status,
        }),
    }
}

/// Return `response` unchanged when it has a success status, or the matching
/// [`Error::HttpStatus`] otherwise.
#[cfg(feature = "http")]
fn ensure_success(response: reqwest::Response, url: &str) -> Result<reqwest::Response> {
    let status = response.status();
    if status.is_success() {
        Ok(response)
    } else {
        Err(Error::HttpStatus {
            url: url.to_owned(),
            status,
        })
    }
}

/// The first byte position advertised by a `Content-Range` header.
///
/// `bytes 100-499/1234` yields `100`; a missing or malformed header yields
/// `None`.
#[cfg(feature = "http")]
fn content_range_start(response: &reqwest::Response) -> Option<u64> {
    response
        .headers()
        .get(reqwest::header::CONTENT_RANGE)?
        .to_str()
        .ok()
        .and_then(parse_content_range)
}

/// Parse the first byte position out of a `Content-Range` value.
///
/// The `bytes` unit is matched case-insensitively, as HTTP tokens are.
fn parse_content_range(value: &str) -> Option<u64> {
    let (unit, range) = value.split_once(' ')?;
    if !unit.eq_ignore_ascii_case("bytes") {
        return None;
    }

    range.split(['-', '/']).next()?.trim().parse().ok()
}

/// Open the temporary file for writing, appending when resuming from `offset`.
#[cfg(feature = "http")]
fn open_partial(path: &Path, offset: u64) -> Result<std::fs::File> {
    let file = if offset == 0 {
        std::fs::File::create(path)
    } else {
        std::fs::OpenOptions::new().append(true).open(path)
    };

    file.map_err(|source| Error::Write {
        path: path.to_path_buf(),
        source,
    })
}

/// Size of an existing partial download, or zero when there is none.
#[cfg(feature = "http")]
fn partial_size(path: &Path) -> u64 {
    std::fs::metadata(path)
        .map(|metadata| metadata.len())
        .unwrap_or(0)
}

/// Stream a response body into `file`, reporting progress.
///
/// `downloaded` seeds the running count with the bytes already on disk so
/// progress and the returned total include a resumed prefix.
#[cfg(feature = "http")]
async fn write_body<F>(
    file: &mut std::fs::File,
    mut response: reqwest::Response,
    dest: &Path,
    url: &str,
    mut downloaded: u64,
    total: Option<u64>,
    progress: &mut F,
) -> Result<u64>
where
    F: FnMut(Progress),
{
    use std::io::Write;

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

    Ok(downloaded)
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

/// The final path segment of a link, ignoring any query or fragment.
fn file_name(link: &str) -> Option<String> {
    Url::parse(link)
        .ok()?
        .path_segments()?
        .next_back()
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
}

/// A stable temporary sibling of `dest`, used both for the in-flight download
/// and to resume an interrupted one on a later run.
#[cfg(any(feature = "http", test))]
fn temp_path(dest: &Path) -> PathBuf {
    let mut name = std::ffi::OsString::from(".");
    name.push(dest.file_name().map_or_else(
        || std::ffi::OsString::from("download"),
        |name| name.to_os_string(),
    ));
    name.push(".part");
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
            paths::cache_dir()
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
        assert_eq!(temp, Path::new("/tmp/.image.tar.xz.part"));
    }

    #[test]
    fn parses_content_range_start() {
        assert_eq!(parse_content_range("bytes 100-499/1234"), Some(100));
        assert_eq!(parse_content_range("Bytes 12-34/*"), Some(12));
        assert_eq!(parse_content_range("bytes 0-0/1"), Some(0));
        assert_eq!(parse_content_range("bytes 12-34/*"), Some(12));
        assert_eq!(parse_content_range("100-499/1234"), None);
        assert_eq!(parse_content_range("bytes */1234"), None);
        assert_eq!(parse_content_range("nonsense"), None);
    }
}
