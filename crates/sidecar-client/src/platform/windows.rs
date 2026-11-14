//! Windows sidecar support (not implemented yet).
//!
//! The intended implementation mirrors Etcher's `lib/shared/sudo/windows.ts`:
//!
//! 1. check the current elevation with `fltmc`;
//! 2. relaunch the helper through `ShellExecuteEx` with the `runas` verb (UAC),
//!    passing the handshake channel through inherited stdio or a named pipe.

use std::path::Path;
use std::process::Child;

use crate::Result;

pub(crate) fn spawn(_program: &Path, _elevate: bool) -> Result<Child> {
    todo!("start the sidecar on Windows")
}
