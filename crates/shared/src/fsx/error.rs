use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to read `{path}`")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write `{path}`")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl Error {
    pub fn read(path: &Path, source: std::io::Error) -> Self {
        Self::Read {
            path: path.to_path_buf(),
            source,
        }
    }

    pub fn write(path: &Path, source: std::io::Error) -> Self {
        Self::Write {
            path: path.to_path_buf(),
            source,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
