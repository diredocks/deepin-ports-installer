//! Resolving what to write, and where.
//!
//! A flash source is either a local image file or a device model that the
//! bundled index maps to an image. Both are resolved into a [`Plan`] that a
//! frontend can show and later execute.

use std::path::PathBuf;

use catalog::{Arch, DeviceIndex, Image, ImageIndex, Query};

use crate::{Error, Result};

/// The image that a flash operation will write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// An image resolved from the bundled index; it may need downloading.
    Remote(Image),

    /// A local image file named by the caller.
    Local {
        /// Path to the image file.
        path: PathBuf,

        /// Size in bytes.
        size: u64,
    },
}

/// Image filters, applied only when the source is a device model.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SourceOptions<'a> {
    /// Architecture filter, needed when a device exists on several of them.
    pub arch: Option<Arch>,

    /// Variant substring filter, e.g. `25-crimson`.
    pub variant: Option<&'a str>,

    /// Publication date (`YYYYMMDD` or `YYYY-MM-DD`) to select.
    pub date: Option<&'a str>,
}

impl SourceOptions<'_> {
    /// Whether any filter is set.
    pub fn is_empty(&self) -> bool {
        self.arch.is_none() && self.variant.is_none() && self.date.is_none()
    }
}

/// What a flash operation would write, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// Image to write.
    pub source: Source,

    /// Block device (or image file) to write to.
    pub target: PathBuf,
}

/// Build the plan for writing `source` to `target`.
///
/// `source` is treated as a local file when it contains a path separator or
/// names an existing file, and as a device model otherwise.
pub fn plan(source: &str, target: impl Into<PathBuf>, options: &SourceOptions<'_>) -> Result<Plan> {
    Ok(Plan {
        source: resolve(source, options)?,
        target: target.into(),
    })
}

/// Resolve what `source` names, without looking at the target.
pub fn resolve(source: &str, options: &SourceOptions<'_>) -> Result<Source> {
    match local_path(source) {
        Some(path) => local(path, options),
        None => remote(source, options),
    }
}

/// The file `source` names, when it names an image rather than a device model.
fn local_path(source: &str) -> Option<PathBuf> {
    let path = PathBuf::from(source);
    let is_file = source.chars().any(std::path::is_separator) || path.is_file();

    is_file.then_some(path)
}

/// Read the metadata of a local image file.
fn local(path: PathBuf, options: &SourceOptions<'_>) -> Result<Source> {
    if !options.is_empty() {
        return Err(Error::FiltersWithLocalSource);
    }

    let metadata = fsx::metadata(&path)?;

    if !metadata.is_file() {
        return Err(Error::NotAFile { path });
    }

    Ok(Source::Local {
        path,
        size: metadata.len(),
    })
}

/// Resolve a device model (or image-group slug) through the embedded index.
fn remote(device: &str, options: &SourceOptions<'_>) -> Result<Source> {
    let images = ImageIndex::embedded()?;
    let devices = DeviceIndex::embedded()?;

    let query = Query {
        device,
        arch: options.arch,
        variant: options.variant,
        date: options.date,
    };

    Ok(Source::Remote(images.resolve(&devices, &query)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use catalog::Candidate;
    use testutils::Workspace;

    fn options<'a>() -> SourceOptions<'a> {
        SourceOptions::default()
    }

    #[test]
    fn resolves_local_files() {
        let root = Workspace::new("deepin-ports-flash", "local");
        let path = root.join("local.img");
        std::fs::write(&path, b"image").unwrap();

        let source = resolve(path.to_str().unwrap(), &options()).unwrap();

        assert_eq!(source, Source::Local { path, size: 5 });
    }

    #[test]
    fn rejects_filters_for_local_files() {
        let root = Workspace::new("deepin-ports-flash", "filtered");
        let path = root.join("filtered.img");
        std::fs::write(&path, b"image").unwrap();

        let options = SourceOptions {
            arch: Some(Arch::Riscv),
            ..SourceOptions::default()
        };

        assert!(matches!(
            resolve(path.to_str().unwrap(), &options),
            Err(Error::FiltersWithLocalSource)
        ));
    }

    #[test]
    fn rejects_directories_and_missing_files() {
        let root = Workspace::new("deepin-ports-flash", "not-a-file");

        assert!(matches!(
            resolve(root.to_str().unwrap(), &options()),
            Err(Error::NotAFile { .. })
        ));

        let missing = root.join("missing/deepin.img");
        assert!(matches!(
            resolve(missing.to_str().unwrap(), &options()),
            Err(Error::Io(_))
        ));
    }

    #[test]
    fn resolves_device_models_through_the_embedded_index() {
        let Source::Remote(image) = resolve("visionfive-2", &options()).unwrap() else {
            panic!("expected a remote image");
        };

        assert!(image.link.contains("-jh7110-"), "{}", image.link);
        assert!(image.size > 0);
    }

    #[test]
    fn reports_unknown_devices() {
        assert!(matches!(
            resolve("no-such-device", &options()),
            Err(Error::Index(catalog::Error::NoImage { .. }))
        ));
    }

    #[test]
    fn reports_ambiguous_devices_with_structured_candidates() {
        // `generic` is published for several architectures.
        let error = resolve("generic", &options()).unwrap_err();
        let Error::Index(catalog::Error::Ambiguous { candidates, .. }) = error else {
            panic!("expected an ambiguous match, got {error}");
        };

        assert!(candidates.len() > 1);
        assert!(candidates.contains(&Candidate {
            arch: Arch::Riscv,
            device: "generic".to_owned(),
        }));
    }

    #[test]
    fn builds_plans() {
        let plan = plan("visionfive-2", "/dev/sdb", &options()).unwrap();

        assert!(matches!(plan.source, Source::Remote(_)));
        assert_eq!(plan.target, PathBuf::from("/dev/sdb"));
    }
}
