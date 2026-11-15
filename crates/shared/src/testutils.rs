//! Test scaffolding shared by the crate test suites.
//!
//! Every suite needs a scratch directory, and the image tests need the same
//! small `image.tar.xz` and checksum manifest. Building them here keeps the
//! fixtures byte-identical across crates and out of the test bodies.
//!
//! This crate is a plain workspace member used only through `dev-dependencies`;
//! nothing shipped links it.

use std::fs;
use std::ops::Deref;
use std::path::{Path, PathBuf};

/// A fresh, empty directory that is removed when it is dropped.
///
/// `prefix` names the crate the test belongs to, so parallel test binaries with
/// a similarly named test do not share a directory.
#[derive(Debug)]
pub struct Workspace {
    path: PathBuf,
}

impl Workspace {
    /// Create a fresh empty workspace for `name` under `prefix`.
    pub fn new(prefix: &str, name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("{prefix}-{}-{name}", std::process::id()));

        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create the test workspace");

        Self { path }
    }

    /// Path of the workspace directory.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Deref for Workspace {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.path
    }
}

impl AsRef<Path> for Workspace {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Build `image.tar.xz` at `root` holding `entries`, and return its path.
pub fn tar_xz(root: &Path, entries: &[(&str, &[u8])]) -> PathBuf {
    let path = root.join("image.tar.xz");
    let mut builder = tar::Builder::new(xz2::write::XzEncoder::new(Vec::new(), 6));

    for (name, contents) in entries {
        let mut header = tar::Header::new_gnu();
        header.set_size(contents.len() as u64);
        header.set_mode(0o644);
        header.set_entry_type(tar::EntryType::Regular);
        builder.append_data(&mut header, name, *contents).unwrap();
    }

    let bytes = builder.into_inner().unwrap().finish().unwrap();
    fs::write(&path, bytes).unwrap();

    path
}

/// The lowercase hex SHA-256 of `contents`.
pub fn sha256(contents: &[u8]) -> String {
    use sha2::Digest as _;

    let mut hasher = sha2::Sha256::new();
    hasher.update(contents);

    hasher
        .finalize()
        .as_slice()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The body of an `image.sha256sum` covering `entries`.
pub fn manifest(entries: &[(&str, &[u8])]) -> Vec<u8> {
    entries
        .iter()
        .map(|(name, contents)| format!("{}  {name}\n", sha256(contents)))
        .collect::<String>()
        .into_bytes()
}
