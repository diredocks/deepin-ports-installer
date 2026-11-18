#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to run `{program}`")]
    Command {
        program: String,
        #[source]
        source: std::io::Error,
    },

    #[error("`{program}` failed with {status}: {stderr}")]
    CommandFailed {
        program: String,
        status: std::process::ExitStatus,
        stderr: String,
    },

    #[error("unexpected `lsblk` output: {0}")]
    LsblkOutput(String),

    #[error("failed to parse `lsblk` JSON output")]
    LsblkJson(#[source] serde_json::Error),

    #[error("listing flash targets is not supported on this platform")]
    UnsupportedPlatform,

    #[error("failed to serialize JSON: {0}")]
    SerializeJson(#[from] serde_json::Error),

    #[error("failed to serialize TOML: {0}")]
    SerializeToml(#[from] toml::ser::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
