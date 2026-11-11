//! The `{path, source}` error pair every file operation reports.

use std::path::{Path, PathBuf};

/// Errors produced while reading or writing files and directories.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A file or directory could not be read.
    #[error("failed to read `{path}`")]
    Read {
        /// Path that could not be read.
        path: PathBuf,

        #[source]
        source: std::io::Error,
    },

    /// A file or directory could not be written.
    #[error("failed to write `{path}`")]
    Write {
        /// Path that could not be written.
        path: PathBuf,

        #[source]
        source: std::io::Error,
    },
}

impl Error {
    /// A read failure on `path`, for use with `map_err`.
    pub fn read(path: &Path, source: std::io::Error) -> Self {
        Self::Read {
            path: path.to_path_buf(),
            source,
        }
    }

    /// A write failure on `path`, for use with `map_err`.
    pub fn write(path: &Path, source: std::io::Error) -> Self {
        Self::Write {
            path: path.to_path_buf(),
            source,
        }
    }
}

/// Convenience alias used across this crate.
pub type Result<T> = std::result::Result<T, Error>;
