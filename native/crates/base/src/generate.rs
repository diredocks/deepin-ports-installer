//! Regenerate the bundled `images.toml` manifest by crawling the Deepin Ports
//! image repository.
//!
//! The repository serves an Apache "fancy" directory index. The crawler walks
//! `<base>/<date>/<arch>/` and turns every `deepin-*.{iso,tar.xz,tar.gz}` file
//! into an [`Image`] record, mirroring `ref/update-image.js`.

use std::collections::BTreeMap;
use std::path::Path;

use astral_html::Document;
use futures_util::stream::{self, StreamExt, TryStreamExt};

use crate::date;
use crate::http;
use crate::{Arch, Error, Image, ImageGroup, ImageIndex, Result};

/// Default repository that hosts the Deepin Ports images.
pub const DEFAULT_BASE: &str = "https://ci.deepin.com/repo/deepin/deepin-ports/cdimage";

/// Default number of concurrent listing requests.
pub const DEFAULT_CONCURRENCY: usize = 4;

/// Settings for [`image_index`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// Base URL of the Deepin Ports image repository.
    pub base: String,

    /// Maximum number of concurrent HTTP requests.
    pub concurrency: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            base: DEFAULT_BASE.to_owned(),
            concurrency: DEFAULT_CONCURRENCY,
        }
    }
}

/// Crawl the repository configured by `options` and build an image index.
pub async fn image_index(options: &Options) -> Result<ImageIndex> {
    let base = options.base.trim_end_matches('/');
    let concurrency = options.concurrency.max(1);

    let dates: Vec<String> = parse_index(&http::get(&format!("{base}/")).await?)?
        .into_iter()
        .filter(is_date_dir)
        .map(|entry| entry.name)
        .collect();

    let arch_lists: Vec<(String, Vec<(Arch, String)>)> = stream::iter(dates)
        .map(|date| async move {
            let entries = parse_index(&http::get(&format!("{base}/{date}")).await?)?;
            let archs = entries
                .into_iter()
                .filter(|entry| entry.dir && entry.name != "../")
                .filter_map(|entry| {
                    let dir = entry.name.trim_end_matches('/').to_owned();
                    arch_from_dir(&dir).map(|arch| (arch, dir))
                })
                .collect();
            Ok::<_, Error>((date, archs))
        })
        .buffer_unordered(concurrency)
        .try_collect()
        .await?;

    let jobs: Vec<(String, Arch, String)> = arch_lists
        .into_iter()
        .flat_map(|(date, archs)| {
            archs
                .into_iter()
                .map(move |(arch, dir)| (date.clone(), arch, dir))
        })
        .collect();

    let mut records: Vec<Record> = stream::iter(jobs)
        .map(|(date, arch, dir)| async move {
            let entries = parse_index(&http::get(&format!("{base}/{date}{dir}/")).await?)?;
            Ok::<_, Error>(
                entries
                    .iter()
                    .filter_map(|entry| record_from(base, arch, &date, &dir, entry))
                    .collect::<Vec<_>>(),
            )
        })
        .buffer_unordered(concurrency)
        .try_collect::<Vec<Vec<Record>>>()
        .await?
        .into_iter()
        .flatten()
        .collect();

    // Sort by link so each device's images keep their chronological (path)
    // order once they are grouped.
    records.sort_by(|a, b| a.image.link.cmp(&b.image.link));

    let mut groups: BTreeMap<(Arch, String), Vec<Image>> = BTreeMap::new();
    for record in records {
        groups
            .entry((record.arch, record.device))
            .or_default()
            .push(record.image);
    }

    let groups: Vec<ImageGroup> = groups
        .into_iter()
        .map(|((arch, device), images)| ImageGroup {
            arch,
            device,
            images,
        })
        .collect();

    Ok(ImageIndex::from(groups))
}

/// Serialize `index` as TOML and write it to `path`.
pub fn write_file(index: &ImageIndex, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    let document = index.to_toml()?;
    std::fs::write(path, document).map_err(|source| Error::Write {
        path: path.to_path_buf(),
        source,
    })
}

/// A single file matched while crawling, before it is grouped.
struct Record {
    arch: Arch,
    device: String,
    image: Image,
}

/// One entry from a directory listing.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    /// Link target as written in the index (a file name or `dir/`).
    name: String,

    /// Whether the entry is a directory.
    dir: bool,

    /// File size in bytes; `0` for directories.
    size: u64,
}

/// Parse an Apache fancy-index page into its entries.
///
/// Every listing line is `<a href="name">…</a> … <date> <time> <size>`: the
/// anchor `href` carries the full name (the link text is truncated) and the
/// trailing token of the same line carries the size. Both are in source order
/// within the `<pre>` listing, so its anchors can be zipped with its text lines.
fn parse_index(html: &str) -> Result<Vec<Entry>> {
    let document = Document::parse(html)?;

    let Some(pre) = document.elements().find(|element| element.is("pre")) else {
        return Ok(Vec::new());
    };

    let names = pre
        .descendants()
        .filter(|element| element.is("a"))
        .filter_map(|element| element.attribute("href"))
        .map(|href| href.value());

    let listing = pre.text();

    Ok(names
        .zip(listing.lines())
        .filter(|(name, _)| *name != "../")
        .map(|(name, line)| Entry {
            dir: name.ends_with('/'),
            size: line
                .split_whitespace()
                .last()
                .and_then(|token| token.parse().ok())
                .unwrap_or(0),
            name: name.to_owned(),
        })
        .collect())
}

/// Whether a listing entry is a `YYYYMMDD/` build directory.
fn is_date_dir(entry: &Entry) -> bool {
    entry.dir && entry.name.strip_suffix('/').is_some_and(date::is_day)
}

/// Map a repository architecture directory to a supported [`Arch`].
///
/// Only `riscv64` differs from the [`Arch`] slug; the rest parse directly.
fn arch_from_dir(dir: &str) -> Option<Arch> {
    match dir {
        "riscv64" => Some(Arch::Riscv),
        other => other.parse().ok(),
    }
}

/// Turn a listing entry into a record, or `None` when it is not an image of
/// `arch_dir`.
fn record_from(
    base: &str,
    arch: Arch,
    date: &str,
    arch_dir: &str,
    entry: &Entry,
) -> Option<Record> {
    let stem = image_stem(&entry.name)?;
    let parts: Vec<&str> = stem.split('-').collect();

    // A deepin image is `deepin-<variant>-<arch>-…`, where the variant is at
    // least three segments; the architecture may be preceded by extras such as
    // the `treeland` suffix.
    let is_image = parts.first() == Some(&"deepin")
        && parts
            .iter()
            .position(|part| *part == arch_dir)
            .is_some_and(|index| index >= 4);
    if !is_image {
        return None;
    }

    let device = if is_archive(&entry.name) {
        device_from(&parts)
    } else {
        "generic".to_owned()
    };

    Some(Record {
        arch,
        device,
        image: Image {
            variant: parts[1..4].join("-"),
            size: entry.size,
            date: date::from_stem(stem).unwrap_or_default(),
            link: format!("{base}/{date}{arch_dir}/{}", entry.name),
        },
    })
}

/// Strip a known image file extension, or `None` for other files.
fn image_stem(name: &str) -> Option<&str> {
    [".tar.xz", ".tar.gz", ".iso"]
        .into_iter()
        .find_map(|extension| name.strip_suffix(extension))
}

/// Whether `name` is a compressed image archive.
fn is_archive(name: &str) -> bool {
    name.ends_with(".tar.xz") || name.ends_with(".tar.gz")
}

/// Derive the device slug from a file name split on `-`.
///
/// Everything between the architecture segment and the trailing date/time pair
/// is the device slug, e.g. `k1-cloud`; an archive without one is `generic`.
fn device_from(parts: &[&str]) -> String {
    parts
        .get(5..parts.len().saturating_sub(2))
        .map(|parts| parts.join("-"))
        .filter(|device| !device.is_empty())
        .unwrap_or_else(|| "generic".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<html>
<head><title>Index of /x/</title></head>
<body bgcolor="white"><pre><a href="../">../</a>
<a href="deepin-25-crimson-immutable-riscv64-20250625-171832.iso">deepin-25-crimson-immutable-riscv64-20250625-17..&gt;</a> 25-Jun-2025 17:24          4893435904
<a href="deepin-25-crimson-preview-riscv64-dp1000-20250625-030117.tar.xz">deepin-25-crimson-preview-riscv64-dp1000-202506..&gt;</a> 25-Jun-2025 13:35          3908305680
<a href="report/">report/</a>                                            25-Jun-2025 17:24                   -
</pre></body></html>"#;

    fn entry(name: &str, size: u64) -> Entry {
        Entry {
            dir: name.ends_with('/'),
            name: name.to_owned(),
            size,
        }
    }

    #[test]
    fn parses_entries_with_sizes_and_directories() {
        let entries = parse_index(SAMPLE).unwrap();
        assert_eq!(entries.len(), 3);

        assert_eq!(
            entries[0].name,
            "deepin-25-crimson-immutable-riscv64-20250625-171832.iso"
        );
        assert_eq!(entries[0].size, 4893435904);
        assert_eq!(entries[1].size, 3908305680);
        assert!(entries[2].dir);
        assert_eq!(entries[2].name, "report/");
    }

    #[test]
    fn date_directories_are_recognized() {
        assert!(is_date_dir(&entry("20250625/", 0)));
        assert!(!is_date_dir(&entry("treeland/", 0)));
        assert!(!is_date_dir(&entry("20250625", 0)));
    }

    #[test]
    fn maps_known_architectures_only() {
        assert_eq!(arch_from_dir("riscv64"), Some(Arch::Riscv));
        assert_eq!(arch_from_dir("arm64"), Some(Arch::Arm64));
        assert_eq!(arch_from_dir("amd64"), Some(Arch::Amd64));
        assert_eq!(arch_from_dir("loong64"), Some(Arch::Loong64));
        assert_eq!(arch_from_dir("mips64el"), None);
    }

    #[test]
    fn builds_device_records() {
        let entry = entry(
            "deepin-25-crimson-preview-riscv64-k1-cloud-20250806-180508.tar.xz",
            4097381076,
        );
        let record = record_from(
            "https://example.com",
            Arch::Riscv,
            "20250806/",
            "riscv64",
            &entry,
        )
        .unwrap();

        assert_eq!(record.arch, Arch::Riscv);
        assert_eq!(record.device, "k1-cloud");
        assert_eq!(record.image.variant, "25-crimson-preview");
        assert_eq!(record.image.date, "20250806-180508");
        assert_eq!(record.image.size, 4097381076);
        assert_eq!(
            record.image.link,
            "https://example.com/20250806/riscv64/deepin-25-crimson-preview-riscv64-k1-cloud-20250806-180508.tar.xz"
        );
    }

    #[test]
    fn isos_are_generic() {
        let entry = entry(
            "deepin-23-beige-preview-riscv64-20250107-145422.iso",
            3621724160,
        );
        let record = record_from(
            "https://example.com",
            Arch::Riscv,
            "20250107/",
            "riscv64",
            &entry,
        )
        .unwrap();

        assert_eq!(record.device, "generic");
        assert_eq!(record.image.variant, "23-beige-preview");
    }

    #[test]
    fn ignores_other_architectures_and_non_images() {
        let arch = "riscv64";
        assert!(
            record_from(
                "https://example.com",
                Arch::Riscv,
                "20240613/",
                arch,
                &entry("deepin-23-beige-preview-arm64-20240613-112412.iso", 1),
            )
            .is_none()
        );
        assert!(
            record_from(
                "https://example.com",
                Arch::Riscv,
                "20250625/",
                arch,
                &entry("md5sum_riscv64.txt", 36),
            )
            .is_none()
        );
        assert!(
            record_from(
                "https://example.com",
                Arch::Riscv,
                "20250625/",
                arch,
                &entry("report/", 0),
            )
            .is_none()
        );
    }

    #[test]
    fn extracts_timestamp_before_trailing_suffixes() {
        let record = record_from(
            "https://example.com",
            Arch::Amd64,
            "20240111/",
            "amd64",
            &entry(
                "deepin-23-beige-beta3-amd64-20240111-115726_kernel-6.7.iso",
                4572413952,
            ),
        )
        .unwrap();

        assert_eq!(record.image.date, "20240111-115726");
        assert_eq!(record.image.variant, "23-beige-beta3");
    }

    #[test]
    fn accepts_variants_with_extra_segments() {
        let record = record_from(
            "https://example.com",
            Arch::Arm64,
            "20240705/",
            "arm64",
            &entry(
                "deepin-23-beige-preview-treeland-arm64-20240705-135729.iso",
                3676411904,
            ),
        )
        .unwrap();

        assert_eq!(record.image.variant, "23-beige-preview");
        assert_eq!(record.device, "generic");
        assert_eq!(record.image.date, "20240705-135729");
    }

    #[test]
    fn writes_and_reloads_file() {
        let index = ImageIndex::parse(
            "[[riscv.jh7110]]\n\
             variant = \"25-crimson-preview\"\n\
             size = 3652822392\n\
             date = \"20250603-113217\"\n\
             link = \"https://example.com/jh7110.tar.xz\"\n",
        )
        .unwrap();
        let path = std::env::temp_dir().join(format!("deepin-images-{}.toml", std::process::id()));

        write_file(&index, &path).unwrap();
        let reloaded = ImageIndex::parse_file(&path).unwrap();
        std::fs::remove_file(&path).unwrap();

        assert_eq!(reloaded, index);
    }
}
