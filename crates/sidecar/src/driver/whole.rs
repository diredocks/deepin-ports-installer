//! `dd-whole` and `iso-write`: stream a single whole-disk or ISO image from offset 0.

use shared::catalog::{Artifact, Role};
use shared::fsx::{self, Amount, BUFFER_SIZE};
use shared::protocol::FlashPlan;
use tokio::fs::{File, OpenOptions};

use super::{Progress, copy};
use crate::{Error, Result};

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
    let mut progress = Progress::new(total, &mut progress);
    copy(
        &mut source,
        &artifact.path,
        &mut target,
        &plan.target,
        &mut buffer,
        None,
        |read| progress.advance(read),
    )
    .await?;

    target
        .sync_all()
        .await
        .map_err(|source| fsx::Error::write(&plan.target, source))?;

    Ok(())
}

fn disk_artifact(artifacts: &[Artifact]) -> Result<&Artifact> {
    match artifacts {
        [artifact] if matches!(artifact.role, Role::Disk | Role::Iso) => Ok(artifact),
        [artifact] => Err(Error::InvalidPlan(format!(
            "dd-whole expects a disk or ISO artifact, got {}",
            artifact.role
        ))),
        artifacts => Err(Error::InvalidPlan(format!(
            "dd-whole expects 1 disk or ISO artifact, got {}",
            artifacts.len()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use shared::catalog::FlashVia;

    use super::*;
    use crate::testutils::TestDir;

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
        let dir = TestDir::new("whole", "write-disk");
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
            Some(&Amount::new(
                payload.len() as u64,
                Some(payload.len() as u64)
            ))
        );
        assert!(reports.windows(2).all(|w| w[0].done < w[1].done));
    }

    #[tokio::test]
    async fn writes_an_iso_like_a_whole_disk() {
        let dir = TestDir::new("whole", "write-iso");
        let image_path = dir.join("image.iso");
        let target_path = dir.join("target.bin");
        let payload = b"bootable iso";
        fs::write(&image_path, payload).unwrap();
        fs::write(&target_path, vec![0u8; payload.len()]).unwrap();

        let job = plan(
            &target_path,
            vec![artifact(&image_path, Role::Iso, payload.len() as u64)],
        );

        write(&job, |_| {}).await.unwrap();

        assert_eq!(fs::read(&target_path).unwrap(), payload);
    }

    #[tokio::test]
    async fn rejects_invalid_artifact_plans() {
        let dir = TestDir::new("whole", "invalid-plan");
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
        let dir = TestDir::new("whole", "missing-target");
        let image_path = dir.join("disk.img");
        let target_path = dir.join("missing.bin");
        fs::write(&image_path, b"whole disk image").unwrap();

        let job = plan(&target_path, vec![artifact(&image_path, Role::Disk, 16)]);
        assert!(matches!(write(&job, |_| {}).await, Err(Error::Fs(_))));
        assert!(!target_path.exists());
    }
}
