/// Errors produced by the unprivileged sidecar client.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The sidecar could not be spawned.
    #[error("failed to start `{program}`")]
    Spawn {
        /// Program that could not be spawned.
        program: String,

        #[source]
        source: std::io::Error,
    },

    /// An I/O operation involving the sidecar failed.
    #[error("sidecar I/O failed")]
    Io(#[source] std::io::Error),

    /// No supported privilege escalation mechanism is available.
    #[error("no supported privilege escalation mechanism is available")]
    ElevationUnavailable,

    /// The sidecar did not announce its WebSocket address.
    #[error("the sidecar exited without announcing its WebSocket address")]
    NoHandshake,

    /// The WebSocket session with the sidecar failed.
    #[error("sidecar websocket failed")]
    WebSocket(#[source] tungstenite::Error),

    /// A sidecar message did not match the protocol.
    #[error("sidecar protocol error: {0}")]
    Protocol(String),

    /// The sidecar refused or failed an operation.
    #[error("sidecar reported an error: {0}")]
    Reported(String),

    /// A request could not be serialized.
    #[error("failed to serialize JSON: {0}")]
    SerializeJson(#[from] serde_json::Error),
}

/// Convenience alias used across this crate.
pub type Result<T> = std::result::Result<T, Error>;
