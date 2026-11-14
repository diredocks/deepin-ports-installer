//! Privileged sidecar process.
//!
//! This binary is spawned through the platform elevation mechanism (`pkexec`
//! on Linux) and runs as root. It binds a loopback WebSocket, announces its
//! address and a per-run token on stdout, and serves the closed set of
//! operations in [`sidecar_protocol::Request`].
//!
//! Writing images is the reason the helper exists: a
//! [`FlashPlan`](sidecar_protocol::flash::FlashPlan) is handed to one of the
//! write drivers, one per `flash_via` method. The drivers are scaffolded only —
//! they document their steps and end in `todo!()`.
//!
//! It deliberately links only [`sidecar_protocol`] and [`catalog`], which it
//! needs to resolve a plan's device group: the download client, the archive
//! handling and the unprivileged client of the protocol are kept out of the root
//! process.

mod driver;
mod error;
mod platform;
mod server;

pub use error::{Error, Result};
pub use server::serve;
