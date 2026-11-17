use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Index(#[from] crate::catalog::Error),

    #[error(transparent)]
    Io(#[from] crate::fsx::Error),

    #[error("`{path}` is not a regular file")]
    NotAFile { path: PathBuf },

    #[error("architecture, variant and date filters only apply when the source is a device model")]
    FiltersWithLocalSource,

    #[error(transparent)]
    Archive(#[from] crate::archive::Error),

    #[error("`{path}` contains no image artifact")]
    NoArtifacts { path: PathBuf },

    #[error("`{name}` is listed in a checksum manifest but missing")]
    Missing { name: String },

    #[error("`{path}` is not covered by any checksum manifest")]
    Unverified { path: PathBuf },

    #[error(transparent)]
    Checksum(#[from] crate::checksum::Error),

    #[error("`{via}` is not a write method of the `{soc}` devices")]
    UnsupportedFlashVia {
        via: crate::catalog::FlashVia,
        soc: String,
    },

    #[error("cannot infer write method from the prepared artifacts; specify `--via`")]
    MissingFlashVia,

    #[error("`{via}` requires a `{role}` image artifact")]
    MissingArtifact {
        via: crate::catalog::FlashVia,
        role: crate::catalog::Role,
    },

    #[error("`{via}` requires extra `{role}` file; pass `--extra {role}=<PATH>`")]
    MissingExtra {
        via: crate::catalog::FlashVia,
        role: crate::catalog::Role,
    },

    #[error("`{via}` does not use extra `{role}` file")]
    UnexpectedExtra {
        via: crate::catalog::FlashVia,
        role: crate::catalog::Role,
    },

    #[error("extra `{role}` file was specified more than once")]
    DuplicateExtra { role: crate::catalog::Role },

    #[error(transparent)]
    Sidecar(#[from] crate::sidecar_client::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
