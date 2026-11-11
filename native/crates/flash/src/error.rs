use std::path::PathBuf;

/// Errors produced while planning or running an installation.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The bundled index could not be loaded or queried.
    #[error(transparent)]
    Index(#[from] catalog::Error),

    /// A local image file could not be read.
    #[error("failed to read image `{path}`")]
    LocalImage {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// A local source is not a regular file.
    #[error("`{path}` is not a regular file")]
    NotAFile {
        /// Source path that was not a file.
        path: PathBuf,
    },

    /// Image filters were given together with a local file source.
    #[error("architecture, variant and date filters only apply when the source is a device model")]
    FiltersWithLocalSource,
}

/// Convenience alias used across this crate.
pub type Result<T> = std::result::Result<T, Error>;
