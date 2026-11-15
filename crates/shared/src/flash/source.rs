//! Resolving which image to flash: a local file or a device model from the index.

use std::path::PathBuf;

use crate::catalog::{Arch, DeviceIndex, FlashVia, Image, ImageIndex, Query};

use crate::flash::{Error, Result};

/// The image a flash operation will write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// An image resolved from the bundled index; it may need downloading.
    Remote {
        /// Selected image record.
        image: Image,

        /// Write methods accepted by the matching device group, in preference order.
        flash_via: Vec<FlashVia>,
    },

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

/// Resolve what `source` names: a local file, or a device model when it is no path.
pub fn resolve(source: &str, options: &SourceOptions<'_>) -> Result<Source> {
    let path = PathBuf::from(source);
    let local_path = (source.chars().any(std::path::is_separator) || path.is_file()).then_some(path);

    match local_path {
        Some(path) => local(path, options),
        None => remote(source, options),
    }
}

/// Read the metadata of a local image file.
fn local(path: PathBuf, options: &SourceOptions<'_>) -> Result<Source> {
    if *options != SourceOptions::default() {
        return Err(Error::FiltersWithLocalSource);
    }

    let metadata = crate::fsx::metadata(&path)?;
    if !metadata.is_file() {
        return Err(Error::NotAFile { path });
    }

    Ok(Source::Local {
        path,
        size: metadata.len(),
    })
}

/// Resolve a device model (or image-group slug) through the embedded index.
pub fn remote(device: &str, options: &SourceOptions<'_>) -> Result<Source> {
    let images = ImageIndex::embedded()?;
    let devices = DeviceIndex::embedded()?;

    let query = Query {
        device,
        arch: options.arch,
        variant: options.variant,
        date: options.date,
    };

    let image = images.resolve(&devices, &query)?;
    let flash_via = devices
        .groups()
        .iter()
        .find(|group| group.arch == image.arch && group.soc.eq_ignore_ascii_case(&image.device))
        .map(|group| group.flash_via.clone())
        .unwrap_or_default();

    Ok(Source::Remote { image, flash_via })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Candidate;
    use crate::testutils::Workspace;

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
        let Source::Remote { image, flash_via } = resolve("visionfive-2", &options()).unwrap()
        else {
            panic!("expected a remote image");
        };

        assert!(image.link.contains("-jh7110-"), "{}", image.link);
        assert!(image.size > 0);
        assert_eq!(flash_via, [FlashVia::DdWhole]);
    }

    #[test]
    fn reports_unknown_devices() {
        assert!(matches!(
            resolve("no-such-device", &options()),
            Err(Error::Index(crate::catalog::Error::NoImage { .. }))
        ));
    }

    #[test]
    fn reports_ambiguous_devices_with_structured_candidates() {
        // `generic` is published for several architectures.
        let error = resolve("generic", &options()).unwrap_err();
        let Error::Index(crate::catalog::Error::Ambiguous { candidates, .. }) = error else {
            panic!("expected an ambiguous match, got {error}");
        };

        assert!(candidates.len() > 1);
        assert!(candidates.contains(&Candidate {
            arch: Arch::Riscv,
            device: "generic".to_owned(),
        }));
    }
}
