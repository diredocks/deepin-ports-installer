//! Wire format shared by the privileged helper and its client.
//!
//! Messages are JSON objects in WebSocket text frames. The operation set is
//! closed on purpose: unknown commands fail to deserialize instead of being
//! forwarded to a shell or to the operating system.

use serde::{Deserialize, Serialize};

/// A request from the unprivileged client.
///
/// Keep this enum small: every variant is code that runs as root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Request {
    /// Placeholder privileged operation: run the platform probe (`id`).
    Id,

    /// Stop the helper.
    Shutdown,
}

/// The helper's reply to [`Request::Id`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Reply {
    /// The operation succeeded.
    Ok {
        /// Captured standard output.
        output: String,
    },

    /// The operation failed.
    Error {
        /// Human readable failure reason.
        message: String,
    },
}

/// Address and token the helper announces on stdout before accepting clients.
///
/// This is the first line of the helper's stdout; the client reads it and uses
/// it to open the WebSocket connection.
#[derive(Serialize, Deserialize)]
pub(crate) struct Handshake {
    /// `host:port` the helper listens on.
    pub(crate) address: String,

    /// Per-run bearer token for the `token` query parameter.
    pub(crate) token: String,
}

impl Handshake {
    /// WebSocket URL the client must connect to.
    ///
    /// Tokens are hex, so they never need percent-encoding here. The server
    /// still parses the query with `form_urlencoded` because that side consumes
    /// untrusted input.
    pub(crate) fn url(&self) -> String {
        format!("ws://{}/?token={}", self.address, self.token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_are_flat_and_closed() {
        assert_eq!(
            serde_json::to_value(Request::Id).unwrap(),
            serde_json::json!({ "op": "id" })
        );
        assert_eq!(
            serde_json::to_value(Request::Shutdown).unwrap(),
            serde_json::json!({ "op": "shutdown" })
        );
        assert!(serde_json::from_str::<Request>(r#"{"op":"run_shell"}"#).is_err());
    }

    #[test]
    fn replies_are_flat() {
        let reply = Reply::Ok {
            output: "uid=0(root)\n".into(),
        };

        assert_eq!(
            serde_json::to_value(&reply).unwrap(),
            serde_json::json!({ "status": "ok", "output": "uid=0(root)\n" })
        );
    }
}
