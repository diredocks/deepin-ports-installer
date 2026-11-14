//! Unprivileged client for the sidecar process.
//!
//! Some operations need root, while the installer itself must stay
//! unprivileged. They are delegated to a small helper process that is started
//! through the platform elevation mechanism (`pkexec` on Linux).
//!
//! The sidecar itself lives in the `deepin-ports-installer-helper` binary; it
//! shares only [`sidecar_protocol`] with this crate, so the root process does
//! not link the client, the image model or anything else here.
//!
//! # Security model
//!
//! The helper listens on a WebSocket bound to `127.0.0.1` and understands only
//! the operations represented by `Request` messages: it never accepts a program
//! name, shell string or path from the client. A local attacker who reaches the
//! socket can only ask for those fixed operations, and must know the per-run
//! token announced on the helper's stdout.
//!
//! Keep the protocol closed. Do not add a generic "run this command" operation.
//!
//! # Protocol
//!
//! 1. The helper binds `127.0.0.1:0` and prints `{ "address", "token" }` on
//!    stdout.
//! 2. The client connects to `ws://<address>/?token=<token>`, sends one JSON
//!    request per text frame, and reads the reply.
//! 3. The helper exits on `shutdown` or when the connection closes.
//!
//! # Example
//!
//! ```no_run
//! use sidecar_client::{Client, LaunchOptions};
//!
//! # fn main() -> sidecar_client::Result<()> {
//! let mut client = Client::launch(LaunchOptions::new("/usr/lib/deepin-ports-installer/helper"))?;
//! println!("{}", client.id()?);
//! client.shutdown()?;
//! # Ok(())
//! # }
//! ```
//!
//! # Platform support
//!
//! Elevation lives in `platform/{linux,macos,windows}.rs`. The shared client
//! compiles on every platform; macOS and Windows currently keep `todo!()`
//! placeholders.

mod client;
mod error;
mod platform;

pub use client::{Client, LaunchOptions};
pub use error::{Error, Result};
