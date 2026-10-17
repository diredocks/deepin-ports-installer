pub mod arch;
pub mod date;
pub mod devices;
pub mod download;
#[cfg(feature = "generate")]
pub mod generate;
#[cfg(feature = "http")]
pub mod http;
pub mod images;
pub mod paths;
pub mod text;

pub use arch::Arch;
pub use devices::{DeviceGroup, DeviceIndex};
pub use images::{Image, ImageGroup, ImageIndex, Query};

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

    /// Writing a file to disk failed.
    #[error("failed to write `{path}`")]
    Write {
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
    #[error("device `{device}` matches multiple image groups; narrow it with `--arch`: {}", .candidates.join(", "))]
    Ambiguous {
        /// Requested device model or image-group slug.
        device: String,

        /// Matched groups, formatted as `<arch>/<device>`.
        candidates: Vec<String>,
    },

    /// The destination already exists and overwriting was not requested.
    #[error("`{path}` already exists (use `--force` to overwrite)")]
    AlreadyExists {
        /// Conflicting destination path.
        path: PathBuf,
    },

    /// An image link does not end in a usable file name.
    #[error("image link `{0}` has no file name")]
    LinkFileName(String),

    /// A download ended before the whole image was written.
    #[cfg(feature = "http")]
    #[error("download of `{url}` ended after {downloaded} of {expected} bytes")]
    Incomplete {
        /// URL that was being downloaded.
        url: String,

        /// Bytes written before the body ended.
        downloaded: u64,

        /// Expected total size in bytes.
        expected: u64,
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

    /// The platform application directories could not be determined.
    #[error("failed to determine the platform application directories")]
    ProjectDirs,

    /// Building the HTTP client used to crawl the image repository failed.
    #[cfg(feature = "http")]
    #[error("failed to build HTTP client")]
    HttpClient(#[source] reqwest::Error),

    /// A listing request failed.
    #[cfg(feature = "http")]
    #[error("request to `{url}` failed")]
    Request {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    /// A listing request returned a non-success status.
    #[cfg(feature = "http")]
    #[error("`{url}` returned HTTP {status}")]
    HttpStatus {
        url: String,
        status: reqwest::StatusCode,
    },

    /// A directory listing is not valid HTML.
    #[cfg(feature = "generate")]
    #[error("invalid HTML listing: {0}")]
    Html(#[from] astral_html::Error),
}

/// Convenience alias used across this crate.
pub type Result<T> = std::result::Result<T, Error>;
