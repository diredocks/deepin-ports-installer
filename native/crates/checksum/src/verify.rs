//! Hashing files and comparing them with manifest entries.

use std::io::Read;
use std::path::Path;

use blake2::Blake2b512;
use md5::Md5;
use sha2::Sha256;

use crate::manifest::Manifest;
use crate::{Algorithm, Digest, Error, Result};

/// Read buffer used while hashing.
const BUFFER_SIZE: usize = 1024 * 1024;

/// The strongest manifest that covers `file`, with the checksum it records.
pub fn manifest_for<'a>(manifests: &'a [Manifest], file: &Path) -> Option<(&'a Manifest, &'a str)> {
    manifests
        .iter()
        .filter_map(|manifest| manifest.digest(file).map(|digest| (manifest, digest)))
        .max_by_key(|(manifest, _)| manifest.algorithm().strength())
}

/// Verify `file` against the strongest manifest covering it.
///
/// Returns the checksum it matched, or `None` when no manifest covers the file.
/// `progress` receives the number of bytes hashed so far in this file.
pub fn verify<F>(file: &Path, manifests: &[Manifest], progress: F) -> Result<Option<Digest>>
where
    F: FnMut(u64),
{
    let Some((manifest, expected)) = manifest_for(manifests, file) else {
        return Ok(None);
    };
    let algorithm = manifest.algorithm();

    let actual = hash(algorithm, file, progress)?;

    if actual != expected {
        return Err(Error::Mismatch {
            path: file.to_path_buf(),
            algorithm,
            expected: expected.to_owned(),
            actual,
        });
    }

    Ok(Some(Digest {
        algorithm,
        value: actual,
    }))
}

/// Compute the digest of `file`, reporting the number of bytes hashed so far.
fn hash<F>(algorithm: Algorithm, file: &Path, progress: F) -> Result<String>
where
    F: FnMut(u64),
{
    match algorithm {
        Algorithm::Md5 => hash_with::<Md5, _>(file, progress),
        Algorithm::Blake2b => hash_with::<Blake2b512, _>(file, progress),
        Algorithm::Sha256 => hash_with::<Sha256, _>(file, progress),
    }
}

fn hash_with<D, F>(file: &Path, mut progress: F) -> Result<String>
where
    D: digest::Digest,
    F: FnMut(u64),
{
    let mut reader = std::fs::File::open(file).map_err(|source| Error::Read {
        path: file.to_path_buf(),
        source,
    })?;

    let mut hasher = D::new();
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

    fn sha256(contents: &[u8]) -> String {
        use digest::Digest as _;

        let mut hasher = Sha256::new();
        hasher.update(contents);
        hasher
            .finalize()
            .as_slice()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    #[test]
    fn computes_matches_coreutils() {
        let root = workspace("coreutils");
        let path = root.join("empty.bin");
        std::fs::write(&path, b"").unwrap();

        let empty = [
            (
                Algorithm::Sha256,
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            (Algorithm::Md5, "d41d8cd98f00b204e9800998ecf8427e"),
            (
                Algorithm::Blake2b,
                "786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce",
            ),
        ];

        for (algorithm, expected) in empty {
            let manifest = Manifest::parse(
                root.join("empty.bin"),
                algorithm,
                &format!("{expected}  empty.bin"),
            );
            let digest = verify(&path, &[manifest], |_| {}).unwrap().unwrap();

            assert_eq!(digest.algorithm, algorithm);
            assert_eq!(digest.value, expected);
        }

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn reports_mismatches() {
        let root = workspace("mismatch");
        let path = root.join("image.img");
        std::fs::write(&path, b"payload").unwrap();

        let manifest = Manifest::parse(
            path.clone(),
            Algorithm::Sha256,
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

        assert_eq!(verify(&path, &[], |_| {}).unwrap(), None);

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn prefers_the_strongest_manifest() {
        let root = workspace("strongest");
        let path = root.join("image.img");
        std::fs::write(&path, b"payload").unwrap();

        let sha = Manifest::parse(
            path.clone(),
            Algorithm::Sha256,
            &format!("{}  image.img", sha256(b"payload")),
        );
        let md5 = Manifest::parse(
            path.clone(),
            Algorithm::Md5,
            &format!("{}  image.img", "0".repeat(32)),
        );

        // The MD5 manifest would fail, the SHA-256 one must win.
        let digest = verify(&path, &[md5, sha], |_| {}).unwrap().unwrap();
        assert_eq!(digest.algorithm, Algorithm::Sha256);

        std::fs::remove_dir_all(&root).unwrap();
    }
}
