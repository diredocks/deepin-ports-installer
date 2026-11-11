//! Unpacking and verifying a downloaded image.

use std::path::{Path, PathBuf};

use checksum::Manifest;

use crate::artifact::{Artifact, Role};
use crate::progress::{Phase, Progress};
use crate::{Error, Result};

/// Settings for [`prepare`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PrepareOptions {
    /// Unpack the archive again instead of reusing a previous extraction.
    pub force: bool,

    /// Accept artifacts that no checksum manifest covers.
    pub allow_unverified: bool,
}

/// An image that is unpacked and verified, ready to be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prepared {
    /// Directory the artifacts live in.
    pub directory: PathBuf,

    /// Artifacts found there, in flash order.
    pub artifacts: Vec<Artifact>,

    /// Artifacts no checksum manifest covered, in flash order.
    ///
    /// Only possible when [`PrepareOptions::allow_unverified`] is set.
    pub unverified: Vec<PathBuf>,
}

// TODO(verification): only manifests already on disk are used, so these stay
// unverified after a plain `download`: plain images, whose mirror checksums live
// in the `sha256sum_<arch>.txt` that `download` no longer fetches, and archives
// carrying no manifest inside, which need `--allow-unverified`. Restoring either
// needs an explicit source for published checksums — the image index is the
// natural one — not a fixed list of probed URLs.

/// Unpack and verify the image at `image`.
///
/// An archive is unpacked into [`archive::Archive::extraction_dir`] unless that
/// directory exists, in which case the previous extraction is reused and
/// verified again; [`PrepareOptions::force`] unpacks it from scratch. A plain
/// image is used in place. Verification is driven by the manifests on disk: an
/// artifact is hashed and compared when one covers it, a file a manifest lists
/// is an [`Error::Missing`] when it is not there, and an artifact no manifest
/// covers is an [`Error::Unverified`] unless
/// [`PrepareOptions::allow_unverified`] is set. Progress is reported through
/// `progress`, one phase at a time. The network is never touched.
pub fn prepare<F>(image: &Path, options: &PrepareOptions, mut progress: F) -> Result<Prepared>
where
    F: FnMut(Progress),
{
    let inputs = match archive::Archive::new(image) {
        Some(archive) => unpack(&archive, options, &mut progress)?,
        None => in_place(image)?,
    };

    assemble(image, inputs, options, progress)
}

/// Files to inspect, and the manifests published next to them.
struct Inputs {
    /// Directory the files live in.
    directory: PathBuf,

    /// Every regular file found there.
    files: Vec<PathBuf>,

    /// Manifests among those files.
    manifests: Vec<Manifest>,
}

/// Unpack `archive`, unless the previous extraction is reused.
fn unpack<F>(
    archive: &archive::Archive<'_>,
    options: &PrepareOptions,
    progress: &mut F,
) -> Result<Inputs>
where
    F: FnMut(Progress),
{
    let directory = archive.extraction_dir();

    if !directory.exists() || options.force {
        archive.extract(&directory, |extracted| {
            progress(Progress {
                phase: Phase::Extract,
                done: extracted.done,
                total: extracted.total,
            });
        })?;
    }

    let files = files(&directory)?;
    let manifests = Manifest::load(files.iter().cloned())?;
    require_listed(&files, &manifests)?;

    Ok(Inputs {
        directory,
        files,
        manifests,
    })
}

/// Refuse an extraction that lacks a file its manifest lists.
///
/// A reused extraction can be incomplete — an interrupted unpack, a deleted
/// artifact — and the manifest inside the archive is the only record of what
/// the image is made of. Only artifacts are required; a manifest may also cover
/// files that are not written to the device.
fn require_listed(files: &[PathBuf], manifests: &[Manifest]) -> Result<()> {
    let listed = manifests
        .iter()
        .flat_map(|manifest| manifest.files())
        .filter(|name| Role::from_file_name(name).is_some());

    for name in listed {
        let present = files
            .iter()
            .any(|path| path.file_name().and_then(|file| file.to_str()) == Some(name));

        if !present {
            return Err(Error::Missing {
                name: name.to_owned(),
            });
        }
    }

    Ok(())
}

/// The plain image itself, with whatever manifests its directory holds.
fn in_place(image: &Path) -> Result<Inputs> {
    let directory = image
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    Ok(Inputs {
        directory: directory.to_path_buf(),
        files: vec![image.to_path_buf()],
        manifests: Manifest::in_directory(directory)?,
    })
}

/// Detect the artifacts among the `inputs` and verify them.
fn assemble<F>(
    image: &Path,
    inputs: Inputs,
    options: &PrepareOptions,
    progress: F,
) -> Result<Prepared>
where
    F: FnMut(Progress),
{
    let Inputs {
        directory,
        files,
        manifests,
    } = inputs;

    let mut artifacts = Vec::new();
    for path in files {
        if let Some(artifact) = Artifact::detect(&path)? {
            artifacts.push(artifact);
        }
    }
    artifacts.sort_by(Artifact::order);

    if artifacts.is_empty() {
        return Err(Error::NoArtifacts {
            path: image.to_path_buf(),
        });
    }

    let unverified = verify(&artifacts, &manifests, options, progress)?;

    Ok(Prepared {
        directory,
        artifacts,
        unverified,
    })
}

/// Check every artifact against the manifest that covers it.
///
/// Returns the artifacts no manifest covered, which the caller may only accept
/// when [`PrepareOptions::allow_unverified`] is set.
fn verify<F>(
    artifacts: &[Artifact],
    manifests: &[Manifest],
    options: &PrepareOptions,
    mut progress: F,
) -> Result<Vec<PathBuf>>
where
    F: FnMut(Progress),
{
    let covers = |artifact: &Artifact| checksum::manifest_for(manifests, &artifact.path).is_some();

    if !options.allow_unverified
        && let Some(artifact) = artifacts.iter().find(|artifact| !covers(artifact))
    {
        return Err(Error::Unverified {
            path: artifact.path.clone(),
        });
    }

    let total: u64 = artifacts
        .iter()
        .filter(|artifact| covers(artifact))
        .map(|artifact| artifact.size)
        .sum();

    let mut unverified = Vec::new();
    let mut done = 0;

    for artifact in artifacts {
        if !covers(artifact) {
            unverified.push(artifact.path.clone());
            continue;
        }

        let base = done;
        checksum::verify(&artifact.path, manifests, |hashed| {
            progress(Progress {
                phase: Phase::Verify,
                done: base + hashed,
                total: Some(total),
            });
        })?;

        done += artifact.size;
    }

    Ok(unverified)
}

/// Every regular file below `root`, sorted by path.
fn files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];

    while let Some(directory) = pending.pop() {
        let entries = std::fs::read_dir(&directory).map_err(|source| Error::Read {
            path: directory.clone(),
            source,
        })?;

        for entry in entries {
            let entry = entry.map_err(|source| Error::Read {
                path: directory.clone(),
                source,
            })?;
            let path = entry.path();

            let file_type = entry.file_type().map_err(|source| Error::Read {
                path: path.clone(),
                source,
            })?;

            if file_type.is_dir() {
                pending.push(path);
            } else if file_type.is_file() {
                files.push(path);
            }
        }
    }

    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Role;

    /// A fresh, empty directory for one test.
    fn workspace(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "deepin-ports-prepare-{}-{name}",
            std::process::id()
        ));

        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    /// Build an `image.tar.xz` holding `entries` at `root`.
    fn archive(root: &Path, entries: &[(&str, &[u8])]) -> PathBuf {
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
        std::fs::write(&path, bytes).unwrap();
        path
    }

    fn sha256(contents: &[u8]) -> String {
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

    fn manifest(entries: &[(&str, &[u8])]) -> Vec<u8> {
        entries
            .iter()
            .map(|(name, contents)| format!("{}  {name}\n", sha256(contents)))
            .collect::<String>()
            .into_bytes()
    }

    #[test]
    fn unpacks_and_verifies_an_archive() {
        let root = workspace("verified");
        let image = b"the root filesystem";
        let manifest = manifest(&[("deepin-k1-riscv64-25-desktop-installer.root.ext4", image)]);

        let archive = archive(
            &root,
            &[
                ("deepin-k1-riscv64-25-desktop-installer.root.ext4", image),
                (
                    "deepin-k1-riscv64-25-desktop-installer.sha256sum",
                    &manifest,
                ),
            ],
        );

        let destination = root.join("image");
        let options = PrepareOptions::default();

        let prepared = prepare(&archive, &options, |_| {}).unwrap();

        assert_eq!(prepared.directory, destination);
        assert!(prepared.unverified.is_empty());

        let artifact = prepared
            .artifacts
            .iter()
            .find(|artifact| artifact.role == Role::Root)
            .unwrap();
        assert_eq!(artifact.size, image.len() as u64);
        assert!(artifact.path.is_file());

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn reuses_an_existing_extraction() {
        let root = workspace("reused");
        let image = b"the root filesystem";
        let manifest = manifest(&[("image.root.ext4", image)]);
        let archive = archive(
            &root,
            &[("image.root.ext4", image), ("image.sha256sum", &manifest)],
        );

        let first = prepare(&archive, &PrepareOptions::default(), |_| {}).unwrap();
        std::fs::remove_file(&archive).unwrap();
        let second = prepare(&archive, &PrepareOptions::default(), |_| {}).unwrap();

        assert_eq!(first.artifacts, second.artifacts);

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn rejects_a_mismatched_checksum() {
        let root = workspace("mismatch");
        let image = b"corrupted";
        let manifest = manifest(&[("image.root.ext4", b"expected")]);
        let archive = archive(
            &root,
            &[("image.root.ext4", image), ("image.sha256sum", &manifest)],
        );

        let options = PrepareOptions::default();

        assert!(matches!(
            prepare(&archive, &options, |_| {}),
            Err(Error::Checksum(checksum::Error::Mismatch { .. }))
        ));

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn requires_a_checksum_unless_allowed() {
        let root = workspace("unverified");
        let archive = archive(&root, &[("image.root.ext4", b"payload")]);

        assert!(matches!(
            prepare(&archive, &PrepareOptions::default(), |_| {}),
            Err(Error::Unverified { .. })
        ));

        let allowed = PrepareOptions {
            allow_unverified: true,
            ..PrepareOptions::default()
        };
        let prepared = prepare(&archive, &allowed, |_| {}).unwrap();

        assert_eq!(prepared.unverified, [prepared.artifacts[0].path.clone()]);

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn rejects_a_manifest_listing_a_missing_artifact() {
        let root = workspace("missing");
        let manifest = manifest(&[("image.root.ext4", b"the root filesystem")]);
        let archive = archive(&root, &[("image.sha256sum", &manifest)]);

        assert!(matches!(
            prepare(&archive, &PrepareOptions::default(), |_| {}),
            Err(Error::Missing { name }) if name == "image.root.ext4"
        ));

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn uses_plain_images_in_place() {
        let root = workspace("plain");
        let image = root.join("deepin-k1-riscv64-25-desktop-installer.img");
        std::fs::write(&image, b"whole disk").unwrap();
        std::fs::write(
            root.join("deepin-k1-riscv64-25-desktop-installer.img.sha256sum"),
            format!(
                "{}  deepin-k1-riscv64-25-desktop-installer.img\n",
                sha256(b"whole disk")
            ),
        )
        .unwrap();

        let prepared = prepare(&image, &PrepareOptions::default(), |_| {}).unwrap();

        assert_eq!(prepared.directory, root);
        assert_eq!(prepared.artifacts.len(), 1);
        assert_eq!(prepared.artifacts[0].role, Role::Disk);
        assert!(prepared.unverified.is_empty());

        std::fs::remove_dir_all(&root).unwrap();
    }
}
