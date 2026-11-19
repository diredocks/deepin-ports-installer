//! Privileged root helper (`deepin-ports-installer-helper`), spawned via
//! platform elevation (`pkexec` on Linux).
//!
//! Links `shared` with `default-features = false` so HTTP/archive/UI code stays
//! out of the root process. Write drivers live in `driver/`.

mod driver;
mod error;
mod server;
#[cfg(test)]
mod testutils;

pub use error::{Error, Result};
pub use server::serve;
