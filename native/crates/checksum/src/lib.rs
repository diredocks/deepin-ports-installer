//! Checksum manifests and verification of downloaded files.
//!
//! Deepin Ports builds publish a `<stem>.sha256sum` and a `<stem>.md5sum` next
//! to the image artifacts inside the archive, and some mirrors publish a
//! `<archive>.b2sum` (or `.sha256`) next to the archive itself. The main mirror
//! also groups checksums per architecture as `sha256sum_<arch>.txt` and
//! `md5sum_<arch>.txt`. Every manifest is a list of `<checksum>  <file>` lines,
//! as written by the matching coreutils tool.
//!
//! The crate knows nothing about images, archives or block devices: it parses
//! manifests, finds them in a directory or among a set of paths, hashes files
//! and compares the results. Callers decide which files must be verified and
//! what an unverified file means.

mod algorithm;
mod manifest;
mod verify;

pub use algorithm::{Algorithm, Digest};
pub use manifest::Manifest;
pub use verify::{manifest_for, verify};

use std::path::PathBuf;

/// Errors produced while reading manifests and hashing files.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A manifest or image file could not be read.
    #[error("failed to read `{path}`")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// A file name does not name a known checksum algorithm.
    #[error("`{path}` is not a known checksum manifest")]
    UnknownManifest {
        /// File whose name does not name an algorithm.
        path: PathBuf,
    },

    /// A file does not match the checksum recorded for it.
    #[error("`{path}` failed {algorithm} verification: expected {expected}, got {actual}")]
    Mismatch {
        /// File that failed verification.
        path: PathBuf,

        /// Algorithm the manifest uses.
        algorithm: Algorithm,

        /// Checksum recorded in the manifest.
        expected: String,

        /// Checksum computed from the file.
        actual: String,
    },
}

/// Convenience alias used across this crate.
pub type Result<T> = std::result::Result<T, Error>;
