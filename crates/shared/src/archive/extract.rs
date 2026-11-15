//! Streaming tar archives through a decompressor into a directory.

use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use crate::fsx::{Mode, Sibling};

use crate::archive::{Error, Progress, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Compression {
    Tar,
    Gzip,
    Xz,
}

// Longest first so `.tar.gz` wins over `.tar`.
const SUFFIXES: [(&str, Compression); 5] = [
    (".tar.xz", Compression::Xz),
    (".tar.gz", Compression::Gzip),
    (".txz", Compression::Xz),
    (".tgz", Compression::Gzip),
    (".tar", Compression::Tar),
];

fn split(name: &str) -> Option<(&str, Compression)> {
    let lower = name.to_ascii_lowercase();
    let (suffix, compression) = SUFFIXES
        .iter()
        .find(|(suffix, _)| lower.ends_with(suffix))?;

    // The suffix is ASCII, so removing its byte length cannot split a
    // multi-byte character of `name`.
    let stem = &name[..name.len() - suffix.len()];

    (!stem.is_empty()).then_some((stem, *compression))
}

pub fn stem(name: &str) -> Option<&str> {
    split(name).map(|(stem, _)| stem)
}

pub fn is_archive(name: &str) -> bool {
    split(name).is_some()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Archive<'a> {
    path: &'a Path,
    stem: &'a str,
    compression: Compression,
}

impl<'a> Archive<'a> {
    pub fn new(path: &'a Path) -> Option<Self> {
        let name = path.file_name()?.to_str()?;
        let (stem, compression) = split(name)?;

        Some(Self {
            path,
            stem,
            compression,
        })
    }

    pub fn path(&self) -> &'a Path {
        self.path
    }

    pub fn extraction_dir(&self) -> PathBuf {
        self.path.with_file_name(self.stem)
    }

    /// Unpack into `destination` atomically via a temporary sibling, rejecting escaping members.
    pub fn extract<F>(&self, destination: &Path, progress: F) -> Result<()>
    where
        F: FnMut(Progress),
    {
        crate::fsx::create_parents(destination)?;

        let temporary = Sibling::new(destination, Mode::Discard);
        let staging = temporary.path();

        if staging.exists() {
            std::fs::remove_dir_all(staging)
                .map_err(|source| crate::fsx::Error::write(staging, source))?;
        }
        std::fs::create_dir_all(staging).map_err(|source| crate::fsx::Error::write(staging, source))?;

        unpack(self.path, self.compression, staging, progress)?;

        temporary.commit_dir(destination)?;
        Ok(())
    }
}

/// Stream the archive into `destination` through its decompressor.
fn unpack<F>(
    archive: &Path,
    compression: Compression,
    destination: &Path,
    progress: F,
) -> Result<()>
where
    F: FnMut(Progress),
{
    let file = crate::fsx::open(archive)?;

    let total = file.metadata().ok().map(|metadata| metadata.len());
    let counted = crate::fsx::Chunked::new(file, total, progress);
    let buffered = BufReader::with_capacity(crate::fsx::BUFFER_SIZE, counted);

    let source: Box<dyn Read + '_> = match compression {
        Compression::Tar => Box::new(buffered),
        Compression::Gzip => Box::new(flate2::read::GzDecoder::new(buffered)),
        Compression::Xz => Box::new(xz2::read::XzDecoder::new(buffered)),
    };

    let mut unpacker = tar::Archive::new(source);
    let entries = unpacker
        .entries()
        .map_err(|source| crate::fsx::Error::read(archive, source))?;

    for entry in entries {
        let mut entry = entry.map_err(|source| crate::fsx::Error::read(archive, source))?;

        let path = entry
            .path()
            .map_err(|source| crate::fsx::Error::read(archive, source))?
            .into_owned();

        if !entry
            .unpack_in(destination)
            .map_err(|source| crate::fsx::Error::write(&path, source))?
        {
            return Err(Error::UnsafeMember { path });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutils::Workspace;

    #[test]
    fn recognizes_archives() {
        for name in [
            "/cache/image.tar.xz",
            "/cache/image.tar.gz",
            "/cache/image.tar",
            "/cache/image.txz",
            "/cache/image.tgz",
            "/cache/IMAGE.TAR.XZ",
        ] {
            assert!(Archive::new(Path::new(name)).is_some(), "{name}");
        }

        for name in [
            "/cache/image.img",
            "/cache/image.iso",
            "/cache/image.xz",
            "/cache/image.zip",
        ] {
            assert!(Archive::new(Path::new(name)).is_none(), "{name}");
        }
    }

    #[test]
    fn derives_the_extraction_directory() {
        let extraction_dir = |name: &str| Archive::new(Path::new(name)).map(|a| a.extraction_dir());

        assert_eq!(
            extraction_dir("/cache/images/deepin-k1-20250625.tar.xz"),
            Some(PathBuf::from("/cache/images/deepin-k1-20250625"))
        );
        assert_eq!(
            extraction_dir("/cache/images/deepin-k1.tar.gz"),
            Some(PathBuf::from("/cache/images/deepin-k1"))
        );
        assert_eq!(extraction_dir("/cache/images/deepin-k1.iso"), None);
        assert_eq!(extraction_dir("/cache/images/.tar.xz"), None);
    }

    #[test]
    fn exposes_archive_suffixes() {
        assert_eq!(stem("image.tar.xz"), Some("image"));
        assert_eq!(stem("image.tgz"), Some("image"));
        assert_eq!(stem("image.tar"), Some("image"));
        assert_eq!(stem("image.iso"), None);
        assert_eq!(stem(".tar"), None);
    }

    #[test]
    fn extracts_archives_and_replaces_the_destination() {
        let root = Workspace::new("deepin-ports-archive", "extract");
        let path = crate::testutils::tar_xz(
            &root,
            &[
                ("image.root.ext4", b"root filesystem"),
                ("image.sha256sum", b"checksums"),
            ],
        );
        let destination = root.join("unpacked");
        let archive = Archive::new(&path).unwrap();

        let mut progress = Vec::new();
        archive
            .extract(&destination, |report| progress.push(report))
            .unwrap();

        assert_eq!(
            std::fs::read(destination.join("image.root.ext4")).unwrap(),
            b"root filesystem"
        );
        assert!(destination.join("image.sha256sum").is_file());
        assert!(progress.iter().all(|report| report.total.is_some()));
        assert_eq!(
            progress.last().unwrap().done,
            std::fs::metadata(&path).unwrap().len()
        );

        // A second extraction replaces the destination instead of merging.
        std::fs::write(destination.join("stale"), b"stale").unwrap();
        archive.extract(&destination, |_| {}).unwrap();
        assert!(!destination.join("stale").exists());
        assert!(!root.join(".unpacked.part").exists());
    }

    #[test]
    fn refuses_members_outside_the_destination() {
        let root = Workspace::new("deepin-ports-archive", "escape");
        let path = root.join("image.tar.xz");
        let mut builder = tar::Builder::new(xz2::write::XzEncoder::new(Vec::new(), 6));

        let mut header = tar::Header::new_gnu();
        header.set_size(1);
        header.set_mode(0o644);
        header.set_entry_type(tar::EntryType::Regular);
        // `Header::set_path` refuses `..`, so write the malicious name directly.
        header.as_old_mut().name[..13].copy_from_slice(b"../escape.img");
        header.set_cksum();
        builder.append(&header, &b"x"[..]).unwrap();

        std::fs::write(&path, builder.into_inner().unwrap().finish().unwrap()).unwrap();

        let destination = root.join("unpacked");
        assert!(matches!(
            Archive::new(&path).unwrap().extract(&destination, |_| {}),
            Err(Error::UnsafeMember { .. })
        ));
        assert!(!root.join("escape.img").exists());
        assert!(!destination.exists());
    }
}
