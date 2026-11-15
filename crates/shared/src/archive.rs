//! Streamed, atomic archive extraction (`.tar.xz`, `.tar.gz`, `.tar`).

mod extract;

pub use extract::{Archive, is_archive, stem};

use std::path::PathBuf;

pub use crate::fsx::Amount as Progress;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] crate::fsx::Error),

    #[error("archive member `{path}` escapes the extraction directory")]
    UnsafeMember { path: PathBuf },
}

pub type Result<T> = std::result::Result<T, Error>;
