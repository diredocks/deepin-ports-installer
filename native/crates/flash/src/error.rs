use std::path::PathBuf;

/// Errors produced while planning, preparing or running an installation.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The bundled index could not be loaded or queried.
    #[error(transparent)]
    Index(#[from] catalog::Error),

    /// An image, archive or directory could not be read or written.
    #[error(transparent)]
    Io(#[from] fsx::Error),

    /// A local source is not a regular file.
    #[error("`{path}` is not a regular file")]
    NotAFile { path: PathBuf },

    /// Image filters were given together with a local file source.
    #[error("architecture, variant and date filters only apply when the source is a device model")]
    FiltersWithLocalSource,

    /// An archive could not be unpacked.
    #[error(transparent)]
    Archive(#[from] archive::Error),

    /// An archive holds no file that can be written to a device.
    #[error("`{path}` contains no image artifact")]
    NoArtifacts { path: PathBuf },

    /// A file a checksum manifest lists is missing from the extraction.
    #[error("`{name}` is listed in a checksum manifest but missing")]
    Missing {
        /// File name as recorded in the manifest.
        name: String,
    },

    /// No checksum manifest covers an artifact.
    #[error("`{path}` is not covered by any checksum manifest")]
    Unverified { path: PathBuf },

    /// A manifest could not be read or an artifact failed verification.
    #[error(transparent)]
    Checksum(#[from] checksum::Error),
}

/// Convenience alias used across this crate.
pub type Result<T> = std::result::Result<T, Error>;
