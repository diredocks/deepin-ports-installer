//! Shared data model of the Deepin Ports installer.
//!
//! This crate is intentionally free of I/O beyond reading the TOML documents it
//! parses: no process spawning, no HTTP, no privileged code. Both frontends
//! (the CLI and a future graphical installer) use it as their source of truth
//! for architectures, devices, images and the artifacts an image unpacks into.

pub mod arch;
pub mod artifact;
pub mod date;
pub mod devices;
pub mod flash;
pub mod images;
mod text;

pub use arch::Arch;
pub use artifact::{Artifact, Role};
pub use devices::{DeviceGroup, DeviceIndex, DeviceQuery};
pub use flash::FlashVia;
pub use images::{Candidate, Image, ImageIndex, ImageQuery, Query};

use std::path::Path;

/// Errors produced while loading the bundled TOML indexes.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Reading a file from disk failed.
    #[error(transparent)]
    Io(#[from] crate::fsx::Error),

    /// No image matched the requested device, variant or date.
    #[error("no image matches device `{device}`")]
    NoImage {
        /// Requested device model or image-group slug.
        device: String,

        variant: Option<String>,

        date: Option<String>,
    },

    /// A device matched more than one image group.
    #[error("device `{device}` matches multiple image groups; specify the architecture: {}", format_candidates(.candidates))]
    Ambiguous {
        /// Requested device model or image-group slug.
        device: String,

        candidates: Vec<Candidate>,
    },

    /// The document is not valid TOML.
    #[error("invalid TOML: {0}")]
    Parse(#[from] toml::de::Error),

    /// Serializing an index to JSON failed.
    #[error("failed to serialize JSON: {0}")]
    SerializeJson(#[from] serde_json::Error),

    /// Serializing an index to TOML failed.
    #[error("failed to serialize TOML: {0}")]
    SerializeToml(#[from] toml::ser::Error),

    /// An architecture slug is not one of the known values.
    #[error("unknown architecture `{0}`")]
    UnknownArch(String),
}

pub type Result<T> = std::result::Result<T, Error>;

fn format_candidates(candidates: &[Candidate]) -> String {
    candidates
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Serialize a device list or image list as pretty JSON.
pub(crate) fn to_json<T: serde::Serialize + ?Sized>(groups: &T) -> Result<String> {
    Ok(serde_json::to_string_pretty(groups)?)
}

pub(crate) fn read_document(path: &Path) -> Result<String> {
    crate::fsx::read_to_string(path).map_err(Error::from)
}
