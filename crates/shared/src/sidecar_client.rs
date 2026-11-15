//! Unprivileged client for the `deepin-ports-installer-helper` sidecar.
//!
//! Protocol: the helper prints `{ "address", "token" }` on stdout, serves
//! `ws://<address>/?token=<token>`, and exits on `shutdown` or disconnect.
//!
//! ```no_run
//! use shared::sidecar_client::{Client, LaunchOptions};
//!
//! # fn main() -> shared::sidecar_client::Result<()> {
//! let mut client = Client::launch(LaunchOptions::new("/usr/lib/deepin-ports-installer/helper"))?;
//! client.shutdown()?;
//! # Ok(())
//! # }
//! ```
//!
//! Note: macOS and Windows elevation remain `todo!()` placeholders.

mod client;
mod error;
mod platform;

pub use client::{Client, LaunchOptions};
pub use error::{Error, Result};
