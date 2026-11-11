//! Platform-specific application directories.
//!
//! Cache and configuration paths are derived from the same
//! [`directories::ProjectDirs`] instance so they follow the conventions of the
//! host platform (XDG on Linux, `~/Library` on macOS, `%LOCALAPPDATA%` on
//! Windows).

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

/// Return the platform default configuration directory.
///
/// On Linux this follows `XDG_CONFIG_HOME`, on macOS it is under
/// `~/Library/Application Support`, and on Windows it is under `%APPDATA%`.
pub fn config_dir() -> Result<PathBuf> {
    project_dirs().map(|dirs| dirs.config_dir().to_path_buf())
}

/// The directory is only resolved, not created; use [`ensure_cache`] when it
/// must exist.
pub fn resolve_cache(override_dir: Option<&Path>) -> Result<PathBuf> {
    override_dir.map_or_else(cache_dir, |path| Ok(path.to_path_buf()))
}

/// The directory is only resolved, not created; use [`ensure_config`] when it
/// must exist.
pub fn resolve_config(override_dir: Option<&Path>) -> Result<PathBuf> {
    override_dir.map_or_else(config_dir, |path| Ok(path.to_path_buf()))
}

pub fn ensure_cache(override_dir: Option<&Path>) -> Result<PathBuf> {
    create_dir(resolve_cache(override_dir)?)
}

pub fn ensure_config(override_dir: Option<&Path>) -> Result<PathBuf> {
    create_dir(resolve_config(override_dir)?)
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

fn create_dir(path: PathBuf) -> Result<PathBuf> {
    std::fs::create_dir_all(&path).map_err(|source| Error::Io {
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_creates_missing_directories() {
        let root = std::env::temp_dir().join(format!(
            "deepin-ports-installer-dirs-{}",
            std::process::id()
        ));

        let cache = ensure_cache(Some(&root.join("cache"))).unwrap();
        let config = ensure_config(Some(&root.join("config"))).unwrap();
        assert!(cache.is_dir());
        assert!(config.is_dir());

        std::fs::remove_dir_all(root).unwrap();
    }
}
