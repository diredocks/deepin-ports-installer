#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to start `{program}`")]
    Spawn {
        program: String,
        #[source]
        source: std::io::Error,
    },

    #[error("sidecar I/O failed")]
    Io(#[source] std::io::Error),

    #[error("no supported privilege escalation mechanism is available")]
    ElevationUnavailable,

    #[error("the sidecar exited without announcing its WebSocket address")]
    NoHandshake,

    #[error("sidecar websocket failed")]
    WebSocket(#[source] tungstenite::Error),

    #[error("sidecar protocol error: {0}")]
    Protocol(String),

    #[error("sidecar reported an error: {0}")]
    Reported(String),

    #[error("failed to serialize JSON: {0}")]
    SerializeJson(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
