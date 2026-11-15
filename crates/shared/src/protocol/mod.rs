//! Wire format shared by the sidecar and its client.
//!
//! Messages are JSON objects in WebSocket text frames. The operation set is
//! closed on purpose: unknown commands fail to deserialize instead of being
//! forwarded to a shell or to the operating system.
//!
//! The values on the wire come from the crates that own them ([`catalog`],
//! [`crate::fsx::Amount`]), so both sides agree without a second copy.

use crate::fsx::Amount;
use serde::{Deserialize, Serialize};

pub mod flash;

use crate::protocol::flash::FlashPlan;

/// A request from the unprivileged client.
///
/// Keep this enum small: every variant is code that runs as root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Request {
    /// Placeholder privileged operation: run the platform probe (`id`).
    Id,

    /// Write the artifacts of `plan` to its target: the only operation that can
    /// destroy data.
    Flash {
        /// Job to execute.
        plan: FlashPlan,
    },

    /// Stop the helper.
    Shutdown,
}

/// The helper's reply to a [`Request`].
///
/// A flash job replies with zero or more [`Reply::Progress`] frames, then exactly
/// one [`Reply::Ok`] or [`Reply::Error`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Reply {
    /// The operation succeeded.
    Ok {
        /// Captured standard output, or a human readable summary.
        output: String,
    },

    /// The operation is still running, and has written `amount` so far.
    Progress {
        /// Bytes written so far and in total.
        amount: Amount,
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
pub struct Handshake {
    /// `host:port` the helper listens on.
    pub address: String,

    /// Per-run bearer token for the `token` query parameter.
    pub token: String,
}

impl Handshake {
    /// WebSocket URL the client must connect to.
    ///
    /// Tokens are hex, so they never need percent-encoding here. The server
    /// still parses the query with `form_urlencoded` because that side consumes
    /// untrusted input.
    pub fn url(&self) -> String {
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
    fn flash_jobs_round_trip_with_their_plan() {
        let request = Request::Flash {
            plan: FlashPlan {
                arch: crate::catalog::Arch::Riscv,
                soc: "jh7110".into(),
                via: crate::catalog::FlashVia::DdWhole,
                target: "/dev/sdb".into(),
                artifacts: vec![crate::catalog::Artifact {
                    path: "/tmp/image.img".into(),
                    role: crate::catalog::Role::Disk,
                    size: 42,
                }],
            },
        };

        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(serde_json::from_str::<Request>(&json).unwrap(), request);
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

        let progress = Reply::Progress {
            amount: Amount::new(512, Some(1024)),
        };
        assert_eq!(
            serde_json::to_value(&progress).unwrap(),
            serde_json::json!({ "status": "progress", "amount": { "done": 512, "total": 1024 } })
        );
    }
}
