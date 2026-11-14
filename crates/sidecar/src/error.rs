/// Errors produced by the sidecar.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A privileged command could not be run.
    #[error("failed to run `{program}`")]
    Command {
        /// Program that could not be spawned.
        program: String,

        #[source]
        source: std::io::Error,
    },

    /// An I/O operation failed, typically while starting the server.
    #[error("sidecar I/O failed")]
    Io(#[source] std::io::Error),

    /// Timed out waiting for the client to connect.
    #[error("timed out waiting for the sidecar client")]
    Timeout,

    /// The WebSocket session with the client failed.
    #[error("sidecar websocket failed")]
    WebSocket(#[source] tungstenite::Error),

    /// A client message did not match the protocol.
    #[error("sidecar protocol error: {0}")]
    Protocol(String),

    /// A message could not be serialized.
    #[error("failed to serialize JSON: {0}")]
    SerializeJson(#[from] serde_json::Error),
}

/// Convenience alias used across this crate.
pub type Result<T> = std::result::Result<T, Error>;
