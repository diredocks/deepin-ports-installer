use std::path::PathBuf;

/// Errors produced while fetching files.
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
    #[error("download of `{url}` ended after {downloaded} of {expected} bytes")]
    Incomplete {
        /// URL that was being downloaded.
        url: String,

        /// Bytes written before the body ended.
        downloaded: u64,

        /// Expected total size in bytes.
        expected: u64,
    },

    /// The platform application directories could not be determined.
    #[error("failed to determine the platform application directories")]
    ProjectDirs,

    /// Building the HTTP client used to talk to the image repository failed.
    #[error("failed to build HTTP client")]
    HttpClient(#[source] reqwest::Error),

    /// A request failed.
    #[error("request to `{url}` failed")]
    Request {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    /// A request returned a non-success status.
    #[error("`{url}` returned HTTP {status}")]
    HttpStatus {
        url: String,
        status: reqwest::StatusCode,
    },
}

/// Convenience alias used across this crate.
pub type Result<T> = std::result::Result<T, Error>;
