//! SHA-256 checksum manifest parsing and file verification.

mod manifest;
mod verify;

pub use manifest::Manifest;
pub use verify::{manifest_for, verify};

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] crate::fsx::Error),

    #[error("`{path}` is not a sha256 checksum manifest")]
    NotAManifest { path: PathBuf },

    #[error("`{path}` failed sha256 verification: expected {expected}, got {actual}")]
    Mismatch {
        path: PathBuf,
        expected: String,
        actual: String,
    },
}

pub type Result<T> = std::result::Result<T, Error>;
