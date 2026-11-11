//! macOS sidecar support (not implemented yet).
//!
//! The intended implementation reads the per-run token from `/dev/urandom` and
//! uses `id` as the placeholder command, like the Linux one.

use crate::Result;

pub(crate) fn random_token() -> Result<String> {
    todo!("generate a sidecar token on macOS")
}

pub(crate) fn id() -> Result<String> {
    todo!("run the placeholder privileged command on macOS")
}
