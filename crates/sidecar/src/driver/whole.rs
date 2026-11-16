//! `dd-whole`: write a single whole-disk image, as `dd if=<img> of=/dev/sdX`.
//!
//! jh7110 (VisionFive 2, MilkV Mars, DC ROMA) and phytiumpi ship one `*.img` that
//! already carries the partition table and the bootloader.

use shared::catalog::{Artifact, Role};
use shared::fsx::{self, Amount, BUFFER_SIZE};
use shared::protocol::FlashPlan;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{Error, Result};

/// Stream a single whole-disk image artifact to `plan.target` from offset 0.
pub(super) async fn write(plan: &FlashPlan, mut progress: impl FnMut(Amount)) -> Result<()> {
    let artifact = disk_artifact(&plan.artifacts)?;

    let mut source = File::open(&artifact.path)
        .await
        .map_err(|source| fsx::Error::read(&artifact.path, source))?;
    let total = source.metadata().await.ok().map(|metadata| metadata.len());

    // Open an existing target without creating or truncating so missing device
    // nodes are rejected and Windows `\\.\PhysicalDriveN` (`OPEN_EXISTING`) works.
    let mut target = OpenOptions::new()
        .write(true)
        .open(&plan.target)
        .await
        .map_err(|source| fsx::Error::write(&plan.target, source))?;

    let mut buffer = vec![0u8; BUFFER_SIZE];
    let mut done = 0u64;
    loop {
        let read = source
            .read(&mut buffer)
            .await
            .map_err(|source| fsx::Error::read(&artifact.path, source))?;

        if read == 0 {
            break;
        }

        target
            .write_all(&buffer[..read])
            .await
            .map_err(|source| fsx::Error::write(&plan.target, source))?;

        done += read as u64;
        progress(Amount::new(done, total));
    }

    target
        .flush()
        .await
        .map_err(|source| fsx::Error::write(&plan.target, source))?;
    target
        .sync_all()
        .await
        .map_err(|source| fsx::Error::write(&plan.target, source))?;

    Ok(())
}

fn disk_artifact(artifacts: &[Artifact]) -> Result<&Artifact> {
    match artifacts {
        [artifact] if artifact.role == Role::Disk => Ok(artifact),
        [artifact] => Err(Error::InvalidPlan(format!(
            "dd-whole expects a disk artifact, got {}",
            artifact.role
        ))),
        artifacts => Err(Error::InvalidPlan(format!(
            "dd-whole expects 1 disk artifact, got {}",
            artifacts.len()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use shared::catalog::FlashVia;

    use super::*;

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "deepin-ports-sidecar-whole-{}-{name}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).expect("create test directory");
            Self { path }
        }

        fn join(&self, name: &str) -> PathBuf {
            self.path.join(name)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn plan(target: &Path, artifacts: Vec<Artifact>) -> FlashPlan {
        FlashPlan {
            via: FlashVia::DdWhole,
            target: target.to_path_buf(),
            artifacts,
        }
    }

    fn artifact(path: &Path, role: Role, size: u64) -> Artifact {
        Artifact {
            path: path.to_path_buf(),
            role,
            size,
        }
    }

    #[tokio::test]
    async fn writes_disk_image_from_offset_zero_without_truncating_and_reports_progress() {
        let dir = TestDir::new("write-disk");
        let image_path = dir.join("disk.img");
        let target_path = dir.join("target.bin");

        let payload: Vec<u8> = (0..(BUFFER_SIZE + 512)).map(|i| (i % 251) as u8).collect();
        fs::write(&image_path, &payload).unwrap();

        let mut initial_target = vec![0xAAu8; payload.len() + 256];
        initial_target[payload.len()..].fill(0x55);
        fs::write(&target_path, &initial_target).unwrap();

        let job = plan(
            &target_path,
            vec![artifact(&image_path, Role::Disk, payload.len() as u64)],
        );

        let mut reports = Vec::new();
        write(&job, |amount| reports.push(amount)).await.unwrap();

        let written = fs::read(&target_path).unwrap();
        assert_eq!(written.len(), initial_target.len());
        assert_eq!(&written[..payload.len()], payload.as_slice());
        assert!(written[payload.len()..].iter().all(|&b| b == 0x55));

        assert!(!reports.is_empty());
        assert_eq!(
            reports.last(),
            Some(&Amount::new(payload.len() as u64, Some(payload.len() as u64)))
        );
        assert!(reports.windows(2).all(|w| w[0].done < w[1].done));
    }

    #[tokio::test]
    async fn rejects_invalid_artifact_plans() {
        let dir = TestDir::new("invalid-plan");
        let target_path = dir.join("target.bin");
        fs::write(&target_path, b"placeholder").unwrap();

        let empty = plan(&target_path, vec![]);
        assert!(matches!(
            write(&empty, |_| {}).await,
            Err(Error::InvalidPlan(_))
        ));

        let wrong_role = plan(
            &target_path,
            vec![artifact(&dir.join("boot.ext4"), Role::Boot, 10)],
        );
        assert!(matches!(
            write(&wrong_role, |_| {}).await,
            Err(Error::InvalidPlan(_))
        ));

        let multiple = plan(
            &target_path,
            vec![
                artifact(&dir.join("a.img"), Role::Disk, 10),
                artifact(&dir.join("b.img"), Role::Disk, 10),
            ],
        );
        assert!(matches!(
            write(&multiple, |_| {}).await,
            Err(Error::InvalidPlan(_))
        ));
    }

    #[tokio::test]
    async fn refuses_to_create_missing_target() {
        let dir = TestDir::new("missing-target");
        let image_path = dir.join("disk.img");
        let target_path = dir.join("missing.bin");
        fs::write(&image_path, b"whole disk image").unwrap();

        let job = plan(&target_path, vec![artifact(&image_path, Role::Disk, 16)]);
        assert!(matches!(write(&job, |_| {}).await, Err(Error::Fs(_))));
        assert!(!target_path.exists());
    }
}
