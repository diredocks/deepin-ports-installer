//! Hashing files and comparing them with manifest entries.

use std::io::Read;
use std::path::Path;

use sha2::{Digest as _, Sha256};

use crate::manifest::Manifest;
use crate::{Error, Result};

/// Read buffer used while hashing.
const BUFFER_SIZE: usize = 1024 * 1024;

/// The manifest that covers `file`, with the checksum it records.
pub fn manifest_for<'a>(manifests: &'a [Manifest], file: &Path) -> Option<(&'a Manifest, &'a str)> {
    manifests
        .iter()
        .find_map(|manifest| manifest.digest(file).map(|digest| (manifest, digest)))
}

/// Verify `file` against the manifest that covers it.
///
/// A file no manifest covers is left alone. `progress` receives the number of
/// bytes hashed so far in this file.
pub fn verify<F>(file: &Path, manifests: &[Manifest], progress: F) -> Result<()>
where
    F: FnMut(u64),
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
fn hash<F>(file: &Path, mut progress: F) -> Result<String>
where
    F: FnMut(u64),
{
    let mut reader = std::fs::File::open(file).map_err(|source| Error::Read {
        path: file.to_path_buf(),
        source,
    })?;

    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; BUFFER_SIZE];
    let mut done = 0;

    loop {
        let read = reader.read(&mut buffer).map_err(|source| Error::Read {
            path: file.to_path_buf(),
            source,
        })?;

        if read == 0 {
            break;
        }

        hasher.update(&buffer[..read]);
        done += read as u64;
        progress(done);
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

    /// A fresh, empty directory for one test.
    fn workspace(name: &str) -> std::path::PathBuf {
        let path =
            std::env::temp_dir().join(format!("deepin-ports-verify-{}-{name}", std::process::id()));

        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn computes_matches_coreutils() {
        let root = workspace("coreutils");
        let path = root.join("empty.bin");
        std::fs::write(&path, b"").unwrap();

        let expected = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        let manifest = Manifest::parse(
            root.join("empty.bin.sha256"),
            &format!("{expected}  empty.bin"),
        );

        verify(&path, &[manifest], |_| {}).unwrap();

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn reports_mismatches() {
        let root = workspace("mismatch");
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

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn leaves_uncovered_files_alone() {
        let root = workspace("uncovered");
        let path = root.join("image.img");
        std::fs::write(&path, b"payload").unwrap();

        assert!(manifest_for(&[], &path).is_none());
        verify(&path, &[], |_| {}).unwrap();

        std::fs::remove_dir_all(&root).unwrap();
    }
}
