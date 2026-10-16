pub mod arch;
pub mod devices;
pub mod images;

pub use arch::Arch;
pub use devices::{DeviceGroup, DeviceIndex};
pub use images::{Image, ImageGroup, ImageIndex};

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

    /// The document is not valid TOML.
    #[error("invalid TOML: {0}")]
    Parse(#[from] toml::de::Error),

    /// The document is valid TOML but not shaped like the expected index.
    #[error("unexpected TOML structure: {0}")]
    Shape(String),

    /// An architecture slug is not one of the known values.
    #[error("unknown architecture `{0}`")]
    UnknownArch(String),
}

/// Convenience alias used across this crate.
pub type Result<T> = std::result::Result<T, Error>;
