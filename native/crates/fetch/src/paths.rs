//! Platform-specific application directories.
//!
//! Cache paths are derived from a [`directories::ProjectDirs`] instance so they
//! follow the conventions of the host platform (XDG on Linux, `~/Library` on
//! macOS, `%LOCALAPPDATA%` on Windows).

use std::path::{Path, PathBuf};

use directories::ProjectDirs;

use crate::{Error, Result};

const QUALIFIER: &str = "";
const ORGANIZATION: &str = "deepin-community";
const APPLICATION: &str = "deepin-ports-installer";

/// Return the platform default cache directory.
///
/// On Linux this follows `XDG_CACHE_HOME`, on macOS it is under
/// `~/Library/Caches`, and on Windows it is under `%LOCALAPPDATA%`.
pub fn cache_dir() -> Result<PathBuf> {
    project_dirs().map(|dirs| dirs.cache_dir().to_path_buf())
}

/// Whether `path` refers to a directory.
///
/// An existing directory matches, as does any path with a trailing separator,
/// even one that does not exist yet.
pub fn is_directory(path: &Path) -> bool {
    path.is_dir() || path.to_string_lossy().ends_with(['/', '\\'])
}

fn project_dirs() -> Result<ProjectDirs> {
    ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION).ok_or(Error::ProjectDirs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_dir(path: PathBuf) -> Result<PathBuf> {
        std::fs::create_dir_all(&path).map_err(|source| Error::Io {
            path: path.clone(),
            source,
        })?;
        Ok(path)
    }

    #[test]
    fn creates_missing_directories() {
        let root = std::env::temp_dir().join(format!(
            "deepin-ports-installer-dirs-{}",
            std::process::id()
        ));

        let cache = create_dir(root.join("cache")).unwrap();
        assert!(cache.is_dir());

        std::fs::remove_dir_all(root).unwrap();
    }
}
