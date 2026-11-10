//! macOS privileged helper support (not implemented yet).
//!
//! The intended implementation mirrors Etcher's `lib/shared/sudo/darwin.ts`:
//!
//! 1. spawn the helper with an `osascript` "do shell script ... with
//!    administrator privileges" prompt, or with a `SMJobBless` privileged
//!    helper installed at build time;
//! 2. read the per-run token from `/dev/urandom`;
//! 3. use `id` as the placeholder command.

use std::path::Path;
use std::process::Child;

use crate::Result;

pub(crate) fn spawn_helper(_program: &Path, _elevate: bool) -> Result<Child> {
    todo!("start the privileged helper on macOS")
}

pub(crate) fn random_token() -> Result<String> {
    todo!("generate a privileged helper token on macOS")
}

pub(crate) fn id() -> Result<String> {
    todo!("run the placeholder privileged command on macOS")
}
