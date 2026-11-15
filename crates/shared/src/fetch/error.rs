use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] crate::fsx::Error),

    #[error("`{path}` already exists (use `--force` to overwrite)")]
    AlreadyExists { path: PathBuf },

    #[error("image link `{0}` has no file name")]
    LinkFileName(String),

    #[error("download of `{url}` ended after {downloaded} of {expected} bytes")]
    Incomplete {
        url: String,
        downloaded: u64,
        expected: u64,
    },

    #[error("failed to determine the platform application directories")]
    ProjectDirs,

    #[error("failed to build HTTP client")]
    HttpClient(#[source] reqwest::Error),

    #[error("request to `{url}` failed")]
    Request {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("`{url}` returned HTTP {status}")]
    HttpStatus {
        url: String,
        status: reqwest::StatusCode,
    },
}

pub type Result<T> = std::result::Result<T, Error>;
