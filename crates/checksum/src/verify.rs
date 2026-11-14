//! Hashing files and comparing them with manifest entries.

use std::io::Read;
use std::path::Path;

use fsx::Amount;
use sha2::{Digest as _, Sha256};

use crate::manifest::Manifest;
use crate::{Error, Result};

/// The manifest that covers `file`, with the checksum it records.
pub fn manifest_for<'a>(manifests: &'a [Manifest], file: &Path) -> Option<(&'a Manifest, &'a str)> {
    manifests
        .iter()
        .find_map(|manifest| manifest.digest(file).map(|digest| (manifest, digest)))
}

/// Verify `file` against the manifest that covers it.
///
/// A file no manifest covers is left alone. `progress` receives the number of
/// bytes hashed so far in this file, and its size as the total.
pub fn verify<F>(file: &Path, manifests: &[Manifest], progress: F) -> Result<()>
where
    F: FnMut(Amount),
{
    let Some((_, expected)) = manifest_for(manifests, file) else {
        return Ok(());
    };

    let actual = hash(file, progress)?;

    if actual != expected {
        return Err(Error::Mismatch {
            path: file.to_path_buf(),
            expected: expected.to_owned(),
            actual,
        });
    }

    Ok(())
}

/// Compute the SHA-256 digest of `file`, reporting the number of bytes hashed
/// so far.
fn hash<F>(file: &Path, progress: F) -> Result<String>
where
    F: FnMut(Amount),
{
    let handle = fsx::open(file)?;
    let total = handle.metadata().ok().map(|metadata| metadata.len());
    let mut reader = fsx::Chunked::new(handle, total, progress);

    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; fsx::BUFFER_SIZE];

    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|source| fsx::Error::read(file, source))?;

        if read == 0 {
            break;
        }

        hasher.update(&buffer[..read]);
    }

    Ok(hex(hasher.finalize().as_slice()))
}

/// Render bytes as lowercase hex.
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use testutils::Workspace;

    #[test]
    fn computes_matches_coreutils() {
        let root = Workspace::new("deepin-ports-checksum", "coreutils");
        let path = root.join("empty.bin");
        std::fs::write(&path, b"").unwrap();

        let expected = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        let manifest = Manifest::parse(
            root.join("empty.bin.sha256"),
            &format!("{expected}  empty.bin"),
        );

        verify(&path, &[manifest], |_| {}).unwrap();
    }

    #[test]
    fn reports_mismatches() {
        let root = Workspace::new("deepin-ports-checksum", "mismatch");
        let path = root.join("image.img");
        std::fs::write(&path, b"payload").unwrap();

        let manifest = Manifest::parse(
            root.join("image.img.sha256"),
            &format!("{}  image.img", "0".repeat(64)),
        );

        assert!(matches!(
            verify(&path, &[manifest], |_| {}),
            Err(Error::Mismatch { .. })
        ));
    }

    #[test]
    fn leaves_uncovered_files_alone() {
        let root = Workspace::new("deepin-ports-checksum", "uncovered");
        let path = root.join("image.img");
        std::fs::write(&path, b"payload").unwrap();

        assert!(manifest_for(&[], &path).is_none());
        verify(&path, &[], |_| {}).unwrap();
    }

    #[test]
    fn reports_the_bytes_hashed() {
        let root = Workspace::new("deepin-ports-checksum", "progress");
        let path = root.join("image.img");
        std::fs::write(&path, b"payload").unwrap();

        let manifest = Manifest::parse(root.join("image.img.sha256"), &"0".repeat(64));
        let mut reports = Vec::new();

        let _ = verify(&path, &[manifest], |amount| reports.push(amount));

        assert_eq!(reports.last(), Some(&Amount::new(7, Some(7))));
        assert!(reports.iter().all(|amount| amount.total == Some(7)));
    }
}
