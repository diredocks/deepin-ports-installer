//! Privileged sidecar process.
//!
//! This binary is spawned through the platform elevation mechanism (`pkexec`
//! on Linux) and runs as root. It binds a loopback WebSocket, announces its
//! address and a per-run token on stdout, and serves the closed set of
//! operations in [`sidecar_protocol::Request`].
//!
//! It deliberately links only [`sidecar_protocol`]: the image model, the
//! download client and the unprivileged client of the protocol are kept out of
//! the root process.

mod error;
mod platform;
mod server;

pub use error::{Error, Result};
pub use server::serve;
