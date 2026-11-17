#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to run `{program}`")]
    Command {
        program: String,
        #[source]
        source: std::io::Error,
    },

    #[error("sidecar I/O failed")]
    Io(#[source] std::io::Error),

    #[error(transparent)]
    Fs(#[from] shared::fsx::Error),

    #[error("invalid flash plan: {0}")]
    InvalidPlan(String),

    #[error("timed out waiting for the sidecar client")]
    Timeout,

    #[error("sidecar websocket failed")]
    WebSocket(#[source] tungstenite::Error),

    #[error("sidecar protocol error: {0}")]
    Protocol(String),

    #[error("failed to serialize JSON: {0}")]
    SerializeJson(#[from] serde_json::Error),

    #[error("failed to generate random bytes: {0}")]
    Random(#[from] getrandom::Error),

    #[error("failed to write GPT partition table: {0}")]
    Gpt(#[from] gptman::Error),

    #[error("fastboot failed: {0}")]
    Fastboot(String),
}

impl From<shared::catalog::Error> for Error {
    fn from(err: shared::catalog::Error) -> Self {
        Self::InvalidPlan(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
