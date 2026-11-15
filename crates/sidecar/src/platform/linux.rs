//! Linux sidecar support.
//!
//! The per-run token comes from the kernel CSPRNG.

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
