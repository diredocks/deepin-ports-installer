//! Platform-specific application directories.
//!
//! Cache and configuration paths are derived from the same
//! [`directories::ProjectDirs`] instance so they follow the conventions of the
//! host platform (XDG on Linux, `~/Library` on macOS, `%LOCALAPPDATA%` on
//! Windows).

use std::path::{Path, PathBuf};

use directories::ProjectDirs;

use crate::{Error, Result};

/// Values used to derive the platform application directories.
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

/// Resolve the cache directory, preferring `override_dir` when provided.
///
/// The directory is only resolved, not created; use [`ensure_cache`] when it
/// must exist.
pub fn resolve_cache(override_dir: Option<&Path>) -> Result<PathBuf> {
    override_dir.map_or_else(cache_dir, |path| Ok(path.to_path_buf()))
}

/// Resolve the configuration directory, preferring `override_dir` when
/// provided.
///
/// The directory is only resolved, not created; use [`ensure_config`] when it
/// must exist.
pub fn resolve_config(override_dir: Option<&Path>) -> Result<PathBuf> {
    override_dir.map_or_else(config_dir, |path| Ok(path.to_path_buf()))
}

/// Resolve the cache directory and create it (including parents) if needed.
pub fn ensure_cache(override_dir: Option<&Path>) -> Result<PathBuf> {
    create_dir(resolve_cache(override_dir)?)
}

/// Resolve the configuration directory and create it (including parents) if
/// needed.
pub fn ensure_config(override_dir: Option<&Path>) -> Result<PathBuf> {
    create_dir(resolve_config(override_dir)?)
}

/// Derive [`ProjectDirs`] for this application.
fn project_dirs() -> Result<ProjectDirs> {
    ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION).ok_or(Error::ProjectDirs)
}

/// Create `path` (including parents) and return it.
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
    fn explicit_cache_override_takes_precedence() {
        let dir = std::env::temp_dir().join("deepin-ports-cache-override");
        assert_eq!(resolve_cache(Some(&dir)).unwrap(), dir);
    }

    #[test]
    fn explicit_config_override_takes_precedence() {
        let dir = std::env::temp_dir().join("deepin-ports-config-override");
        assert_eq!(resolve_config(Some(&dir)).unwrap(), dir);
    }

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
