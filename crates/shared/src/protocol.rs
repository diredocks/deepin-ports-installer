//! Wire format shared by the sidecar and its client.
//!
//! The operation set is closed on purpose: unknown commands fail to deserialize
//! instead of being forwarded to a shell or OS.

use crate::fsx::Amount;
use serde::{Deserialize, Serialize};

pub mod flash;

use crate::protocol::flash::FlashPlan;

/// Keep this enum small: every variant is code that runs as root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Request {
    /// Placeholder privileged probe (`id`).
    Id,
    Flash { plan: FlashPlan },
    Shutdown,
}

/// A flash job replies with zero or more `Progress` frames, then one `Ok` or `Error`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Reply {
    Ok { output: String },
    Progress { amount: Amount },
    Error { message: String },
}

/// First line printed on helper stdout so the client can open the WebSocket.
#[derive(Serialize, Deserialize)]
pub struct Handshake {
    pub address: String,
    pub token: String,
}

impl Handshake {
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
