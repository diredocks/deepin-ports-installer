//! Windows sidecar support (not implemented yet).
//!
//! The intended implementation generates the per-run token with
//! `BCryptGenRandom`.

use crate::Result;

pub(crate) fn random_token() -> Result<String> {
    todo!("generate a sidecar token on Windows")
}
