//! Unpacking downloaded image archives.
//!
//! [`Archive`] recognizes the tar archives Deepin Ports publishes (`.tar.xz`,
//! `.tar.gz`, `.tar`) from their file name and streams them into a directory.
//! Decompression is streamed, members outside the destination are refused, and
//! the result is moved into place atomically. The crate knows nothing about
//! images or block devices.

mod extract;

pub use extract::{Archive, Progress};

use std::path::PathBuf;

/// Errors produced while unpacking an archive.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An archive could not be read or unpacked.
    #[error("failed to read `{path}`")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// An extracted file or directory could not be written.
    #[error("failed to write `{path}`")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// An archive member would be extracted outside the destination.
    #[error("archive member `{path}` escapes the extraction directory")]
    UnsafeMember {
        /// Member path as recorded in the archive.
        path: PathBuf,
    },
}

/// Convenience alias used across this crate.
pub type Result<T> = std::result::Result<T, Error>;
