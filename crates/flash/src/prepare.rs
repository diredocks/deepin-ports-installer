//! Unpacking and verifying a downloaded image.

use std::path::{Path, PathBuf};

use checksum::Manifest;
use fsx::Amount;

use crate::artifact::{Artifact, Role};
use crate::progress::{Phase, Progress};
use crate::{Error, Result};

/// Settings for [`prepare`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PrepareOptions {
    /// Unpack the archive again instead of reusing a previous extraction.
    pub force: bool,

    /// Skip verification, accepting whatever the image contains.
    pub skip_verify: bool,
}

/// An image that is unpacked and verified, ready to be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prepared {
    /// Directory the artifacts live in.
    pub directory: PathBuf,

    /// Artifacts found there, in flash order.
    pub artifacts: Vec<Artifact>,

    /// Artifacts that were not hashed against a checksum manifest, in flash
    /// order: those no manifest covered, or every one when
    /// [`PrepareOptions::skip_verify`] is set.
    pub unverified: Vec<PathBuf>,
}

// TODO(verification): only manifests already on disk are used, so these stay
// unverified after a plain `download`: plain images, whose mirror checksums live
// in the `sha256sum_<arch>.txt` that `download` no longer fetches, and archives
// carrying no manifest inside, which need `--skip-verify`. Restoring either
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
/// covers is an [`Error::Unverified`] — unless
/// [`PrepareOptions::skip_verify`] is set, in which case nothing is hashed at
/// all. Progress is reported through `progress`, one phase at a time. The
/// network is never touched.
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
        archive.extract(&directory, |amount| {
            progress(Progress {
                phase: Phase::Extract,
                amount,
            });
        })?;
    }

    let files = fsx::files_below(&directory)?;
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
    let directory = fsx::parent_or_current(image);

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
/// Returns the artifacts that were not hashed: those no manifest covered, or
/// every artifact when [`PrepareOptions::skip_verify`] is set.
fn verify<F>(
    artifacts: &[Artifact],
    manifests: &[Manifest],
    options: &PrepareOptions,
    mut progress: F,
) -> Result<Vec<PathBuf>>
where
    F: FnMut(Progress),
{
    if options.skip_verify {
        return Ok(artifacts
            .iter()
            .map(|artifact| artifact.path.clone())
            .collect());
    }

    let covers = |artifact: &Artifact| checksum::manifest_for(manifests, &artifact.path).is_some();

    if let Some(artifact) = artifacts.iter().find(|artifact| !covers(artifact)) {
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
        checksum::verify(&artifact.path, manifests, |amount| {
            progress(Progress {
                phase: Phase::Verify,
                amount: Amount::new(base + amount.done, Some(total)),
            });
        })?;

        done += artifact.size;
    }

    Ok(unverified)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Role;
    use testutils::{Workspace, manifest, sha256, tar_xz};

    #[test]
    fn unpacks_and_verifies_an_archive() {
        let root = Workspace::new("deepin-ports-flash", "verified");
        let image = b"the root filesystem";
        let manifest = manifest(&[("deepin-k1-riscv64-25-desktop-installer.root.ext4", image)]);

        let archive = tar_xz(
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
    }

    #[test]
    fn reuses_an_existing_extraction() {
        let root = Workspace::new("deepin-ports-flash", "reused");
        let image = b"the root filesystem";
        let manifest = manifest(&[("image.root.ext4", image)]);
        let archive = tar_xz(
            &root,
            &[("image.root.ext4", image), ("image.sha256sum", &manifest)],
        );

        let first = prepare(&archive, &PrepareOptions::default(), |_| {}).unwrap();
        std::fs::remove_file(&archive).unwrap();
        let second = prepare(&archive, &PrepareOptions::default(), |_| {}).unwrap();

        assert_eq!(first.artifacts, second.artifacts);
    }

    #[test]
    fn rejects_a_mismatched_checksum() {
        let root = Workspace::new("deepin-ports-flash", "mismatch");
        let image = b"corrupted";
        let manifest = manifest(&[("image.root.ext4", b"expected")]);
        let archive = tar_xz(
            &root,
            &[("image.root.ext4", image), ("image.sha256sum", &manifest)],
        );

        let options = PrepareOptions::default();

        assert!(matches!(
            prepare(&archive, &options, |_| {}),
            Err(Error::Checksum(checksum::Error::Mismatch { .. }))
        ));
    }

    #[test]
    fn requires_a_checksum_by_default() {
        let root = Workspace::new("deepin-ports-flash", "unverified");
        let archive = tar_xz(&root, &[("image.root.ext4", b"payload")]);

        assert!(matches!(
            prepare(&archive, &PrepareOptions::default(), |_| {}),
            Err(Error::Unverified { .. })
        ));

        let skipped = PrepareOptions {
            skip_verify: true,
            ..PrepareOptions::default()
        };
        let prepared = prepare(&archive, &skipped, |_| {}).unwrap();

        assert_eq!(prepared.unverified, [prepared.artifacts[0].path.clone()]);
    }

    #[test]
    fn skips_a_mismatched_checksum_when_asked() {
        let root = Workspace::new("deepin-ports-flash", "skip-mismatch");
        let image = b"corrupted";
        let manifest = manifest(&[("image.root.ext4", b"expected")]);
        let archive = tar_xz(
            &root,
            &[("image.root.ext4", image), ("image.sha256sum", &manifest)],
        );

        let skipped = PrepareOptions {
            skip_verify: true,
            ..PrepareOptions::default()
        };
        let prepared = prepare(&archive, &skipped, |_| {}).unwrap();

        assert_eq!(prepared.unverified, [prepared.artifacts[0].path.clone()]);
    }

    #[test]
    fn rejects_a_manifest_listing_a_missing_artifact() {
        let root = Workspace::new("deepin-ports-flash", "missing");
        let manifest = manifest(&[("image.root.ext4", b"the root filesystem")]);
        let archive = tar_xz(&root, &[("image.sha256sum", &manifest)]);

        assert!(matches!(
            prepare(&archive, &PrepareOptions::default(), |_| {}),
            Err(Error::Missing { name }) if name == "image.root.ext4"
        ));
    }

    #[test]
    fn uses_plain_images_in_place() {
        let root = Workspace::new("deepin-ports-flash", "plain");
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

        assert_eq!(prepared.directory, root.path());
        assert_eq!(prepared.artifacts.len(), 1);
        assert_eq!(prepared.artifacts[0].role, Role::Disk);
        assert!(prepared.unverified.is_empty());
    }
}
