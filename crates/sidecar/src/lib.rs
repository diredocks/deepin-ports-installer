//! Privileged sidecar process.
//!
//! This binary is spawned through the platform elevation mechanism (`pkexec`
//! on Linux) and runs as root. It binds a loopback WebSocket, announces its
//! address and a per-run token on stdout, and serves the closed set of
//! operations in [`shared::protocol::Request`].
//!
//! Writing images is the reason the helper exists: a
//! [`FlashPlan`](shared::protocol::flash::FlashPlan) goes to one of the write
//! drivers, none of which is implemented yet.
//!
//! It links only [`sidecar_protocol`] and [`catalog`]: the download client and the
//! unprivileged client are kept out of the root process.

mod driver;
mod error;
mod platform;
mod server;

pub use error::{Error, Result};
pub use server::serve;
