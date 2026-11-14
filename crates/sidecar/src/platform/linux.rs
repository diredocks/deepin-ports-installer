//! Linux sidecar support.
//!
//! The per-run token comes from the kernel CSPRNG.

use std::process::Command;

use crate::{Error, Result};

/// Generate a 256 bit token from `/dev/urandom`, hex encoded.
pub(crate) fn random_token() -> Result<String> {
    use std::io::Read;

    let mut bytes = [0u8; 32];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut bytes))
        .map_err(Error::Io)?;

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
