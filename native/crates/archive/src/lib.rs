//! Unpacking downloaded image archives.
//!
//! [`Archive`] recognizes the tar archives Deepin Ports publishes (`.tar.xz`,
//! `.tar.gz`, `.tar`) from their file name and streams them into a directory.
//! Decompression is streamed, members outside the destination are refused, and
//! the result is moved into place atomically. The crate knows nothing about
//! images or block devices.

mod extract;

pub use extract::{Archive, is_archive, stem};

use std::path::PathBuf;

/// Progress reported while unpacking: compressed bytes consumed so far.
pub use fsx::Amount as Progress;

/// Errors produced while unpacking an archive.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An archive or a directory could not be read or written.
    #[error(transparent)]
    Io(#[from] fsx::Error),

    /// An archive member would be extracted outside the destination.
    #[error("archive member `{path}` escapes the extraction directory")]
    UnsafeMember {
        /// Member path as recorded in the archive.
        path: PathBuf,
    },
}

/// Convenience alias used across this crate.
pub type Result<T> = std::result::Result<T, Error>;
