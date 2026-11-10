//! Windows privileged helper support (not implemented yet).
//!
//! The intended implementation mirrors Etcher's `lib/shared/sudo/windows.ts`:
//!
//! 1. check the current elevation with `fltmc`;
//! 2. relaunch the helper through `ShellExecuteEx` with the `runas` verb (UAC),
//!    passing the handshake channel through inherited stdio or a named pipe;
//! 3. generate the per-run token with `BCryptGenRandom`;
//! 4. use `whoami` as the placeholder command.

use std::path::Path;
use std::process::Child;

use crate::Result;

pub(crate) fn spawn_helper(_program: &Path, _elevate: bool) -> Result<Child> {
    todo!("start the privileged helper on Windows")
}

pub(crate) fn random_token() -> Result<String> {
    todo!("generate a privileged helper token on Windows")
}

pub(crate) fn id() -> Result<String> {
    todo!("run the placeholder privileged command on Windows")
}
