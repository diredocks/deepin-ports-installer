//! Linux sidecar support.
//!
//! Elevation goes through `pkexec` (with `--disable-internal-agent`, like
//! Etcher) unless the installer already runs as root.

use std::path::Path;
use std::process::{Child, Command, Stdio};

use crate::{Error, Result};

/// Candidate paths of the polkit elevation program.
const PKEXEC_CANDIDATES: &[&str] = &["/usr/bin/pkexec", "/run/wrappers/bin/pkexec"];

/// Spawn the helper, through `pkexec` unless we are already root.
pub(crate) fn spawn(program: &Path, elevate: bool) -> Result<Child> {
    // SAFETY: `geteuid` has no preconditions and cannot fail.
    let elevate = elevate && unsafe { libc::geteuid() } != 0;

    let launcher = if elevate {
        pkexec().ok_or(Error::ElevationUnavailable)?
    } else {
        program
    };

    let mut command = Command::new(launcher);
    if elevate {
        command.arg("--disable-internal-agent").arg(program);
    }

    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|source| Error::Spawn {
            program: launcher.display().to_string(),
            source,
        })
}

/// First installed `pkexec` candidate.
fn pkexec() -> Option<&'static Path> {
    PKEXEC_CANDIDATES
        .iter()
        .map(Path::new)
        .find(|path| path.is_file())
}
