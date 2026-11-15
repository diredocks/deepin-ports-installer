//! Platform-specific application directories.
//!
//! Cache paths are derived from a [`directories::ProjectDirs`] instance so they
//! follow the conventions of the host platform (XDG on Linux, `~/Library` on
//! macOS, `%LOCALAPPDATA%` on Windows).

use std::path::{Path, PathBuf};

use directories::ProjectDirs;

use crate::fetch::{Error, Result};

const QUALIFIER: &str = "";
const ORGANIZATION: &str = "deepin-community";
const APPLICATION: &str = "deepin-ports-installer";

/// Return the platform default cache directory.
///
/// On Linux this follows `XDG_CACHE_HOME`, on macOS it is under
/// `~/Library/Caches`, and on Windows it is under `%LOCALAPPDATA%`.
pub fn cache_dir() -> Result<PathBuf> {
    ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION)
        .map(|dirs| dirs.cache_dir().to_path_buf())
        .ok_or(Error::ProjectDirs)
}

/// Whether `path` refers to a directory.
///
/// An existing directory matches, as does any path with a trailing separator,
/// even one that does not exist yet.
pub fn is_directory(path: &Path) -> bool {
    path.is_dir() || path.to_string_lossy().ends_with(['/', '\\'])
}
