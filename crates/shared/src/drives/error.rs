/// Errors produced while enumerating local block devices.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Spawning a discovery command failed.
    #[error("failed to run `{program}`")]
    Command {
        /// Program that could not be spawned.
        program: String,

        #[source]
        source: std::io::Error,
    },

    /// A discovery command exited unsuccessfully.
    #[error("`{program}` failed with {status}: {stderr}")]
    CommandFailed {
        /// Program that failed.
        program: String,

        /// Exit status reported by the program.
        status: std::process::ExitStatus,

        /// Captured standard error, trimmed.
        stderr: String,
    },

    /// A discovery command printed unexpected output.
    #[error("unexpected `lsblk` output: {0}")]
    LsblkOutput(String),

    /// `lsblk` printed JSON that could not be parsed.
    #[error("failed to parse `lsblk` JSON output")]
    LsblkJson(#[source] serde_json::Error),

    /// Target enumeration is not implemented for this platform.
    #[error("listing flash targets is not supported on this platform")]
    UnsupportedPlatform,

    /// Serializing the targets to JSON failed.
    #[error("failed to serialize JSON: {0}")]
    SerializeJson(#[from] serde_json::Error),

    /// Serializing the targets to TOML failed.
    #[error("failed to serialize TOML: {0}")]
    SerializeToml(#[from] toml::ser::Error),
}

/// Convenience alias used across this crate.
pub type Result<T> = std::result::Result<T, Error>;
