//! macOS sidecar support (not implemented yet).
//!
//! The intended implementation reads the per-run token from `/dev/urandom`,
//! like the Linux one.

use crate::Result;

pub(crate) fn random_token() -> Result<String> {
    todo!("generate a sidecar token on macOS")
}
