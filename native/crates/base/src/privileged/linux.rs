//! Linux privileged helper support.
//!
//! Elevation goes through `pkexec` (with `--disable-internal-agent`, like
//! Etcher) unless the installer already runs as root. The per-run token comes
//! from the kernel CSPRNG.

use std::path::Path;
use std::process::{Child, Command, Stdio};

use crate::{Error, Result};

/// Path of the polkit elevation program.
const PKEXEC: &str = "/usr/bin/pkexec";

/// Spawn the helper, through `pkexec` unless we are already root.
pub(crate) fn spawn_helper(program: &Path, elevate: bool) -> Result<Child> {
    // SAFETY: `geteuid` has no preconditions and cannot fail.
    let elevate = elevate && unsafe { libc::geteuid() } != 0;
    let launcher = if elevate { Path::new(PKEXEC) } else { program };

    if elevate && !launcher.is_file() {
        return Err(Error::HelperElevationUnavailable);
    }

    let mut command = Command::new(launcher);
    if elevate {
        command.arg("--disable-internal-agent").arg(program);
    }

    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|source| Error::HelperSpawn {
            program: launcher.display().to_string(),
            source,
        })
}

/// Generate a 256 bit token from `/dev/urandom`, hex encoded.
pub(crate) fn random_token() -> Result<String> {
    use std::io::Read;

    let mut bytes = [0u8; 32];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut bytes))
        .map_err(Error::HelperIo)?;

    use std::fmt::Write as _;
    let mut token = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        // Writing to a `String` cannot fail.
        let _ = write!(token, "{byte:02x}");
    }
    Ok(token)
}

/// Run the placeholder privileged command.
///
/// `pkexec` sanitizes the environment, so resolving `id` through `PATH` is not
/// controlled by the unprivileged caller.
pub(crate) fn id() -> Result<String> {
    let output = Command::new("id")
        .output()
        .map_err(|source| Error::Command {
            program: "id".to_owned(),
            source,
        })?;

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
