//! Windows sidecar support (not implemented yet).
//!
//! The intended implementation generates the per-run token with
//! `BCryptGenRandom` and uses `whoami` as the placeholder command.

use crate::Result;

pub(crate) fn random_token() -> Result<String> {
    todo!("generate a sidecar token on Windows")
}

pub(crate) fn id() -> Result<String> {
    todo!("run the placeholder privileged command on Windows")
}
