//! Streaming tar archives through a decompressor into a directory.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use crate::{Error, Result};

/// Read buffer used while unpacking.
const BUFFER_SIZE: usize = 1024 * 1024;

/// Compression applied to a tar archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Compression {
    Tar,
    Gzip,
    Xz,
}

/// Archive suffixes this crate unpacks, longest first so `.tar.gz` wins over
/// `.tar`.
const SUFFIXES: [(&str, Compression); 5] = [
    (".tar.xz", Compression::Xz),
    (".tar.gz", Compression::Gzip),
    (".txz", Compression::Xz),
    (".tgz", Compression::Gzip),
    (".tar", Compression::Tar),
];

/// Work done while unpacking an archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    /// Compressed bytes consumed so far.
    pub done: u64,

    /// Size of the archive in bytes, when known.
    pub total: Option<u64>,
}

/// Direction of the I/O operation an [`Error`] describes.
#[derive(Debug, Clone, Copy)]
enum IoKind {
    Read,
    Write,
}

/// Map an I/O error on `path` to the matching [`Error`], for use with `map_err`.
fn io_error(path: &Path, kind: IoKind) -> impl FnOnce(std::io::Error) -> Error {
    let path = path.to_path_buf();
    move |source| match kind {
        IoKind::Read => Error::Read { path, source },
        IoKind::Write => Error::Write { path, source },
    }
}

/// A recognized tar archive.
///
/// Compression and the default extraction directory are derived from the file
/// name once, in [`Archive::new`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Archive<'a> {
    path: &'a Path,
    stem: &'a str,
    compression: Compression,
}

impl<'a> Archive<'a> {
    /// Recognize `path` as an archive this crate unpacks, or `None`.
    pub fn new(path: &'a Path) -> Option<Self> {
        let name = path.file_name()?.to_str()?;
        let lower = name.to_ascii_lowercase();
        let (suffix, compression) = SUFFIXES
            .iter()
            .find(|(suffix, _)| lower.ends_with(suffix))?;

        // The suffix is ASCII, so removing its byte length cannot split a
        // multi-byte character of `name`.
        let stem = &name[..name.len() - suffix.len()];

        (!stem.is_empty()).then_some(Self {
            path,
            stem,
            compression: *compression,
        })
    }

    /// Path of the archive.
    pub fn path(&self) -> &'a Path {
        self.path
    }

    /// The default extraction directory: a sibling named after the archive with
    /// the archive suffix removed.
    pub fn extraction_dir(&self) -> PathBuf {
        self.path.with_file_name(self.stem)
    }

    /// Unpack into `destination`, replacing it when it exists.
    ///
    /// The archive is unpacked into a temporary sibling that is moved into place
    /// once complete, so `destination` never holds a partial extraction. Members
    /// outside `destination` are refused.
    pub fn extract<F>(&self, destination: &Path, progress: F) -> Result<()>
    where
        F: FnMut(Progress),
    {
        if let Some(parent) = destination
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent).map_err(io_error(parent, IoKind::Write))?;
        }

        let temporary = temporary(destination);
        if temporary.exists() {
            std::fs::remove_dir_all(&temporary).map_err(io_error(&temporary, IoKind::Write))?;
        }
        std::fs::create_dir_all(&temporary).map_err(io_error(&temporary, IoKind::Write))?;

        if let Err(error) = unpack(self.path, self.compression, &temporary, progress) {
            let _ = std::fs::remove_dir_all(&temporary);
            return Err(error);
        }

        if destination.exists() {
            std::fs::remove_dir_all(destination).map_err(io_error(destination, IoKind::Write))?;
        }

        std::fs::rename(&temporary, destination).map_err(io_error(destination, IoKind::Write))
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
    let file = File::open(archive).map_err(io_error(archive, IoKind::Read))?;

    let total = file.metadata().ok().map(|metadata| metadata.len());
    let counted = Counting {
        inner: file,
        done: 0,
        total,
        progress,
    };
    let buffered = BufReader::with_capacity(BUFFER_SIZE, counted);

    let source: Box<dyn Read + '_> = match compression {
        Compression::Tar => Box::new(buffered),
        Compression::Gzip => Box::new(flate2::read::GzDecoder::new(buffered)),
        Compression::Xz => Box::new(xz2::read::XzDecoder::new(buffered)),
    };

    let mut unpacker = tar::Archive::new(source);
    let entries = unpacker
        .entries()
        .map_err(io_error(archive, IoKind::Read))?;

    for entry in entries {
        let mut entry = entry.map_err(io_error(destination, IoKind::Read))?;

        let path = entry
            .path()
            .map_err(io_error(destination, IoKind::Read))?
            .into_owned();

        if !entry
            .unpack_in(destination)
            .map_err(io_error(&path, IoKind::Read))?
        {
            return Err(Error::UnsafeMember { path });
        }
    }

    Ok(())
}

/// A temporary sibling of `destination`, used until unpacking completes.
fn temporary(destination: &Path) -> PathBuf {
    let mut name = std::ffi::OsString::from(".");
    name.push(
        destination
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("extract")),
    );
    name.push(".part");

    destination.with_file_name(name)
}

/// A reader that reports how much of the archive has been consumed.
struct Counting<R, F> {
    inner: R,
    done: u64,
    total: Option<u64>,
    progress: F,
}

impl<R: Read, F: FnMut(Progress)> Read for Counting<R, F> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let read = self.inner.read(buffer)?;

        if read > 0 {
            self.done += read as u64;
            (self.progress)(Progress {
                done: self.done,
                total: self.total,
            });
        }

        Ok(read)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh, empty directory for one test.
    fn workspace(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "deepin-ports-archive-{}-{name}",
            std::process::id()
        ));

        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    /// Build an `image.tar.xz` holding `entries` at `root`.
    fn archive(root: &Path, entries: &[(&str, &[u8])]) -> PathBuf {
        let path = root.join("image.tar.xz");
        let mut builder = tar::Builder::new(xz2::write::XzEncoder::new(Vec::new(), 6));

        for (name, contents) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(contents.len() as u64);
            header.set_mode(0o644);
            header.set_entry_type(tar::EntryType::Regular);
            builder.append_data(&mut header, name, *contents).unwrap();
        }

        let bytes = builder.into_inner().unwrap().finish().unwrap();
        std::fs::write(&path, bytes).unwrap();
        path
    }

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
    fn temporary_directory_is_a_sibling() {
        assert_eq!(
            temporary(Path::new("/cache/images/deepin-k1")),
            PathBuf::from("/cache/images/.deepin-k1.part")
        );
    }

    #[test]
    fn extracts_archives_and_replaces_the_destination() {
        let root = workspace("extract");
        let path = archive(
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
        assert!(!temporary(&destination).exists());

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn refuses_members_outside_the_destination() {
        let root = workspace("escape");
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

        std::fs::remove_dir_all(&root).unwrap();
    }
}
