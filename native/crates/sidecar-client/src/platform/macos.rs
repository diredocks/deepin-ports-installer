//! macOS sidecar support (not implemented yet).
//!
//! The intended implementation mirrors Etcher's `lib/shared/sudo/darwin.ts`:
//!
//! 1. spawn the helper with an `osascript` "do shell script ... with
//!    administrator privileges" prompt, or with a `SMJobBless` privileged
//!    helper installed at build time.

use std::path::Path;
use std::process::Child;

use crate::Result;

pub(crate) fn spawn(_program: &Path, _elevate: bool) -> Result<Child> {
    todo!("start the sidecar on macOS")
}
