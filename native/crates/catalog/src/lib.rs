//! Shared data model of the Deepin Ports installer.
//!
//! This crate is intentionally free of I/O beyond reading the TOML documents it
//! parses: no process spawning, no HTTP, no privileged code. Both frontends
//! (the CLI and a future graphical installer) use it as their source of truth
//! for architectures, devices and images.

pub mod arch;
pub mod date;
pub mod devices;
pub mod images;
mod text;

pub use arch::Arch;
pub use devices::{DeviceGroup, DeviceIndex, DeviceQuery};
pub use images::{Candidate, Image, ImageFilter, ImageGroup, ImageIndex, ImageQuery, Query};

use std::path::PathBuf;

/// Errors produced while loading the bundled TOML indexes.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Reading a file from disk failed.
    #[error("failed to read `{path}`")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// No image matched the requested device, variant or date.
    #[error("no image matches device `{device}`")]
    NoImage {
        /// Requested device model or image-group slug.
        device: String,

        /// Requested variant filter, if any.
        variant: Option<String>,

        /// Requested publication date, if any.
        date: Option<String>,
    },

    /// A device matched more than one image group.
    #[error("device `{device}` matches multiple image groups; narrow it with `--arch`: {}", format_candidates(.candidates))]
    Ambiguous {
        /// Requested device model or image-group slug.
        device: String,

        /// Matched groups, each with its architecture and device slug.
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

/// Convenience alias used across this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Render ambiguous candidates as `<arch>/<device>` for error messages.
fn format_candidates(candidates: &[Candidate]) -> String {
    candidates
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}
