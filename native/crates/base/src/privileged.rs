//! Privileged helper sidecar.
//!
//! Some operations need root, while the installer itself must stay
//! unprivileged. They are delegated to a small helper process that is started
//! through the platform elevation mechanism (`pkexec` on Linux).
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
//! use base::privileged::{Client, LaunchOptions};
//!
//! # fn main() -> base::Result<()> {
//! let mut client = Client::launch(LaunchOptions::new("/usr/lib/deepin-ports-installer/helper"))?;
//! println!("{}", client.id()?);
//! client.shutdown()?;
//! # Ok(())
//! # }
//! ```
//!
//! # Platform support
//!
//! Elevation and token generation live in `privileged/{linux,macos,windows}.rs`.
//! The shared protocol, client and server compile on every platform; macOS and
//! Windows currently keep `todo!()` placeholders.

mod client;
mod protocol;
mod server;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

// Client and server only see a single, target-specific `platform` module.
#[cfg(target_os = "linux")]
use linux as platform;
#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(target_os = "windows")]
use windows as platform;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
compile_error!("the privileged helper only supports Linux, macOS and Windows");

pub use client::{Client, LaunchOptions};
pub use server::serve;
