//! Temporary sibling paths, committed into place once complete.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use crate::{Error, Result};

/// What happens to an uncommitted sibling when it is dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Leave it behind, so a later run can resume from it.
    Keep,

    /// Remove it, so a failed run leaves nothing behind.
    Discard,
}

/// A hidden temporary sibling of a destination path.
///
/// The path is `.<name>.part` next to the destination, so both live on the same
/// filesystem and [`commit_file`](Self::commit_file) /
/// [`commit_dir`](Self::commit_dir) can move it into place with a single
/// `rename`. The destination therefore never holds a partial download or
/// extraction.
///
/// Dropping a sibling that was never committed applies its [`Mode`]: a
/// resumable download keeps its part file, an extraction removes its staging
/// directory.
#[derive(Debug)]
pub struct Sibling {
    path: PathBuf,
    mode: Mode,
    committed: bool,
}

impl Sibling {
    /// The sibling of `destination`.
    ///
    /// A destination without a file name of its own (the filesystem root, or a
    /// path ending in `..`) falls back to `.part` in its place.
    pub fn new(destination: &Path, mode: Mode) -> Self {
        let mut name = OsString::from(".");
        name.push(
            destination
                .file_name()
                .unwrap_or_else(|| OsStr::new("part")),
        );
        name.push(".part");

        Self {
            path: destination.with_file_name(name),
            mode,
            committed: false,
        }
    }

    /// Path of the temporary file or directory.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Replace `destination` with this file.
    pub fn commit_file(mut self, destination: &Path) -> Result<()> {
        if destination.exists() {
            std::fs::remove_file(destination)
                .map_err(|source| Error::write(destination, source))?;
        }

        rename(&self.path, destination)?;
        self.committed = true;
        Ok(())
    }

    /// Replace `destination` with this directory.
    pub fn commit_dir(mut self, destination: &Path) -> Result<()> {
        if destination.exists() {
            std::fs::remove_dir_all(destination)
                .map_err(|source| Error::write(destination, source))?;
        }

        rename(&self.path, destination)?;
        self.committed = true;
        Ok(())
    }
}

fn rename(from: &Path, to: &Path) -> Result<()> {
    std::fs::rename(from, to).map_err(|source| Error::write(to, source))
}

impl Drop for Sibling {
    fn drop(&mut self) {
        if self.committed || self.mode == Mode::Keep {
            return;
        }

        // A staging directory and a part file are removed the same way, but the
        // call has to match what is actually on disk.
        let _ = match std::fs::symlink_metadata(&self.path) {
            Ok(metadata) if metadata.is_dir() => std::fs::remove_dir_all(&self.path),
            _ => std::fs::remove_file(&self.path),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testutils::Workspace;

    #[test]
    fn names_the_sibling_after_the_destination() {
        assert_eq!(
            Sibling::new(Path::new("/tmp/image.tar.xz"), Mode::Keep).path(),
            Path::new("/tmp/.image.tar.xz.part")
        );
        assert_eq!(
            Sibling::new(Path::new("/cache/images/deepin-k1"), Mode::Discard).path(),
            Path::new("/cache/images/.deepin-k1.part")
        );
    }

    #[test]
    fn commits_files_and_directories_into_place() {
        let root = Workspace::new("deepin-ports-fsx", "commit");

        let file = Sibling::new(&root.join("image.img"), Mode::Keep);
        std::fs::write(file.path(), b"new").unwrap();
        std::fs::write(root.join("image.img"), b"old").unwrap();
        file.commit_file(&root.join("image.img")).unwrap();

        assert_eq!(std::fs::read(root.join("image.img")).unwrap(), b"new");
        assert!(!root.join(".image.img.part").exists());

        let directory = Sibling::new(&root.join("unpacked"), Mode::Discard);
        std::fs::create_dir_all(directory.path()).unwrap();
        std::fs::write(directory.path().join("artifact"), b"payload").unwrap();
        std::fs::create_dir_all(root.join("unpacked")).unwrap();
        directory.commit_dir(&root.join("unpacked")).unwrap();

        assert_eq!(
            std::fs::read(root.join("unpacked/artifact")).unwrap(),
            b"payload"
        );
        assert!(!root.join(".unpacked.part").exists());
    }

    #[test]
    fn discards_or_keeps_an_uncommitted_sibling() {
        let root = Workspace::new("deepin-ports-fsx", "drop");

        let kept = Sibling::new(&root.join("kept.img"), Mode::Keep);
        std::fs::write(kept.path(), b"partial").unwrap();
        drop(kept);
        assert!(root.join(".kept.img.part").exists());

        let discarded = Sibling::new(&root.join("discarded"), Mode::Discard);
        std::fs::create_dir_all(discarded.path()).unwrap();
        drop(discarded);
        assert!(!root.join(".discarded.part").exists());
    }

    #[test]
    fn removes_a_discarded_directory_when_the_commit_fails() {
        let root = Workspace::new("deepin-ports-fsx", "failed-commit");

        // The destination is a file, so replacing it with a directory fails
        // after the staging directory was built.
        std::fs::write(root.join("unpacked"), b"file").unwrap();
        let sibling = Sibling::new(&root.join("unpacked"), Mode::Discard);
        std::fs::create_dir_all(sibling.path()).unwrap();

        assert!(sibling.commit_dir(&root.join("unpacked")).is_err());
        assert!(root.join("unpacked").is_file());
        assert!(!root.join(".unpacked.part").exists());
    }
}
