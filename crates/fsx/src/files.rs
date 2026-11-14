//! Locating, creating, reading, writing and listing files.

use std::fs::File;
use std::path::{Path, PathBuf};

use crate::{Error, Result};

/// The parent of `path`, or `.` when it has none.
///
/// A bare file name (`image.img`) lives in the current directory, and so does a
/// path that is already relative to it.
pub fn parent_or_current(path: &Path) -> &Path {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    }
}

/// Create the parent directories of `destination`, if it has any.
///
/// A bare file name has no parent directories to create, and a path that
/// already exists is left alone.
pub fn create_parents(destination: &Path) -> Result<()> {
    let Some(parent) = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    else {
        return Ok(());
    };

    std::fs::create_dir_all(parent).map_err(|source| Error::write(parent, source))
}

/// Open `path` for reading.
pub fn open(path: &Path) -> Result<File> {
    File::open(path).map_err(|source| Error::read(path, source))
}

/// Read the metadata of `path`.
pub fn metadata(path: &Path) -> Result<std::fs::Metadata> {
    std::fs::metadata(path).map_err(|source| Error::read(path, source))
}

/// Read `path` into a string.
pub fn read_to_string(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).map_err(|source| Error::read(path, source))
}

/// Write `contents` to `path`, replacing whatever was there.
pub fn write(path: &Path, contents: impl AsRef<[u8]>) -> Result<()> {
    std::fs::write(path, contents).map_err(|source| Error::write(path, source))
}

/// Every regular file directly inside `directory`, sorted by path.
///
/// Subdirectories are not descended into; use [`files_below`] for a tree.
pub fn files_in(directory: &Path) -> Result<Vec<PathBuf>> {
    collect(directory, false)
}

/// Every regular file below `root`, sorted by path.
///
/// The walk starts at `root` itself and follows no symlinks: an entry counts as
/// a directory only when its own file type says so.
pub fn files_below(root: &Path) -> Result<Vec<PathBuf>> {
    collect(root, true)
}

/// Walk `root`, collecting regular files, optionally descending into
/// subdirectories.
fn collect(root: &Path, recurse: bool) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];

    while let Some(directory) = pending.pop() {
        let entries =
            std::fs::read_dir(&directory).map_err(|source| Error::read(&directory, source))?;

        for entry in entries {
            let entry = entry.map_err(|source| Error::read(&directory, source))?;
            let path = entry.path();

            let file_type = entry
                .file_type()
                .map_err(|source| Error::read(&path, source))?;

            if file_type.is_dir() {
                if recurse {
                    pending.push(path);
                }
            } else if file_type.is_file() {
                files.push(path);
            }
        }
    }

    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use testutils::Workspace;

    #[test]
    fn finds_the_parent_or_the_current_directory() {
        assert_eq!(parent_or_current(Path::new("image.img")), Path::new("."));
        assert_eq!(parent_or_current(Path::new("/a/b")), Path::new("/a"));
    }

    #[test]
    fn creates_missing_parents() {
        let root = Workspace::new("deepin-ports-fsx", "parents");
        let destination = root.join("a/b/image.img");

        create_parents(&destination).unwrap();
        assert!(root.join("a/b").is_dir());

        // A bare file name has no parent directory to create.
        create_parents(Path::new("image.img")).unwrap();
    }

    #[test]
    fn lists_files_without_descending_by_default() {
        let root = Workspace::new("deepin-ports-fsx", "listing");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(root.join("b.img"), b"b").unwrap();
        std::fs::write(root.join("a.img"), b"a").unwrap();
        std::fs::write(root.join("sub/c.img"), b"c").unwrap();

        assert_eq!(
            files_in(&root).unwrap(),
            [root.join("a.img"), root.join("b.img")]
        );
        assert_eq!(
            files_below(&root).unwrap(),
            [
                root.join("a.img"),
                root.join("b.img"),
                root.join("sub/c.img")
            ]
        );
    }

    #[test]
    fn reports_unreadable_directories() {
        let root = Workspace::new("deepin-ports-fsx", "missing");
        let missing = root.join("absent");

        assert!(matches!(files_in(&missing), Err(Error::Read { .. })));
        assert!(matches!(files_below(&missing), Err(Error::Read { .. })));
    }
}
