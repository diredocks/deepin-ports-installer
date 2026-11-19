//! Partitioned block-device writes (`dd-boot-root`, `dd-efi-root`, `dd-root`, `dd-th1520`, `dd-a733`):
//! lay out a GPT partition table, then write each partition image and any extra bootloader.

mod a733;
mod th1520;

use std::io::{Seek, SeekFrom};
use std::path::{Path, PathBuf};

use gptman::{GPT, GPTPartitionEntry};
use shared::catalog::{Artifact, FlashVia, Role};
use shared::fsx::{self, Amount, BUFFER_SIZE};
use shared::protocol::FlashPlan;
use tokio::fs::File;
use tokio::io::AsyncSeekExt;
use uuid::{Builder, Uuid};

use super::{Progress, copy};
use crate::{Error, Result};

const SECTOR_SIZE: u64 = 512;
const MIB_SECTORS: u64 = (1024 * 1024) / SECTOR_SIZE;
const GIB_SECTORS: u64 = 1024 * MIB_SECTORS;
const MIN_GPT_SECTORS: u64 = 68;

const TYPE_LINUX_FS: [u8; 16] =
    Uuid::from_u128(0x0fc63daf_8483_4772_8e79_3d69d8477de4).to_bytes_le();
const TYPE_EFI_SYSTEM: [u8; 16] =
    Uuid::from_u128(0xc12a7328_f81f_11d2_ba4b_00a0c93ec93b).to_bytes_le();
const TYPE_LINUX_SWAP: [u8; 16] =
    Uuid::from_u128(0x0657fd6d_a4ab_43c4_84e5_0933c84b4f4f).to_bytes_le();

struct PartitionSpec {
    name: &'static str,
    type_guid: [u8; 16],
    starting_lba: u64,
    ending_lba: u64,
}

impl PartitionSpec {
    fn new(
        name: &'static str,
        type_guid: [u8; 16],
        (starting_lba, ending_lba): (u64, u64),
    ) -> Self {
        Self {
            name,
            type_guid,
            starting_lba,
            ending_lba,
        }
    }
}

struct WriteSpec {
    sector: u64,
    max_bytes: u64,
    path: PathBuf,
}

impl WriteSpec {
    fn partition((start, end): (u64, u64), artifact: &Artifact) -> Self {
        Self {
            sector: start,
            max_bytes: (end - start + 1) * SECTOR_SIZE,
            path: artifact.path.clone(),
        }
    }

    fn region(sector: u64, sectors: u64, artifact: &Artifact) -> Self {
        Self {
            sector,
            max_bytes: sectors * SECTOR_SIZE,
            path: artifact.path.clone(),
        }
    }
}

struct Layout {
    partitions: Vec<PartitionSpec>,
    writes: Vec<WriteSpec>,
}

struct TwoParts {
    role: Role,
    name: &'static str,
    type_guid: [u8; 16],
    min_sectors: u64,
}

pub(super) async fn write(plan: &FlashPlan, mut progress: impl FnMut(Amount)) -> Result<()> {
    let (file, writes) = partition_target(plan).await?;
    write_regions(&plan.target, file, &writes, &mut progress).await
}

async fn partition_target(plan: &FlashPlan) -> Result<(std::fs::File, Vec<WriteSpec>)> {
    let plan = plan.clone();

    tokio::task::spawn_blocking(move || -> Result<(std::fs::File, Vec<WriteSpec>)> {
        let mut target = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&plan.target)
            .map_err(|source| fsx::Error::write(&plan.target, source))?;

        let disk_bytes = target
            .seek(SeekFrom::End(0))
            .map_err(|source| fsx::Error::write(&plan.target, source))?;
        if disk_bytes < MIN_GPT_SECTORS * SECTOR_SIZE {
            return Err(Error::InvalidPlan(format!(
                "target `{}` ({disk_bytes} bytes) is too small for a GPT partition table",
                plan.target.display()
            )));
        }

        GPT::write_protective_mbr_into(&mut target, SECTOR_SIZE)?;
        let mut gpt = GPT::new_from(&mut target, SECTOR_SIZE, random_guid()?)?;

        let Layout { partitions, writes } = build_layout(
            plan.via,
            &plan.artifacts,
            gpt.header.first_usable_lba,
            gpt.header.last_usable_lba,
        )?;

        for (index, part) in partitions.into_iter().enumerate() {
            let number = u32::try_from(index + 1)
                .map_err(|_| Error::InvalidPlan("too many partitions".into()))?;
            gpt[number] = GPTPartitionEntry {
                partition_type_guid: part.type_guid,
                unique_partition_guid: random_guid()?,
                starting_lba: part.starting_lba,
                ending_lba: part.ending_lba,
                attribute_bits: 0,
                partition_name: part.name.into(),
            };
        }

        gpt.write_into(&mut target)?;
        Ok((target, writes))
    })
    .await
    .map_err(|err| Error::Io(std::io::Error::other(err)))?
}

async fn write_regions(
    target_path: &Path,
    file: std::fs::File,
    writes: &[WriteSpec],
    progress: &mut impl FnMut(Amount),
) -> Result<()> {
    let mut total = 0u64;
    for spec in writes {
        let actual = tokio::fs::metadata(&spec.path)
            .await
            .map_err(|source| fsx::Error::read(&spec.path, source))?
            .len();
        if actual > spec.max_bytes {
            return Err(Error::InvalidPlan(format!(
                "`{}` ({actual} bytes) exceeds allocated region ({} bytes)",
                spec.path.display(),
                spec.max_bytes
            )));
        }
        total += actual;
    }

    let mut target = File::from_std(file);
    let mut progress = Progress::new(Some(total), progress);
    let mut buffer = vec![0u8; BUFFER_SIZE];

    for spec in writes {
        target
            .seek(SeekFrom::Start(spec.sector * SECTOR_SIZE))
            .await
            .map_err(|source| fsx::Error::write(target_path, source))?;

        let mut source = File::open(&spec.path)
            .await
            .map_err(|source| fsx::Error::read(&spec.path, source))?;

        copy(
            &mut source,
            &spec.path,
            &mut target,
            target_path,
            &mut buffer,
            Some(spec.max_bytes),
            |read| progress.advance(read),
        )
        .await?;
    }

    target
        .sync_all()
        .await
        .map_err(|source| fsx::Error::write(target_path, source))?;

    Ok(())
}

fn build_layout(
    via: FlashVia,
    artifacts: &[Artifact],
    first_usable: u64,
    last_usable: u64,
) -> Result<Layout> {
    match via {
        FlashVia::DdBootRoot => layout_two_partitions(
            artifacts,
            first_usable,
            last_usable,
            TwoParts {
                role: Role::Boot,
                name: "boot",
                type_guid: TYPE_LINUX_FS,
                min_sectors: 2 * GIB_SECTORS,
            },
        ),
        FlashVia::DdEfiRoot => layout_two_partitions(
            artifacts,
            first_usable,
            last_usable,
            TwoParts {
                role: Role::Efi,
                name: "efi",
                type_guid: TYPE_EFI_SYSTEM,
                min_sectors: 512 * MIB_SECTORS,
            },
        ),
        FlashVia::DdRoot => layout_root(artifacts, first_usable, last_usable),
        FlashVia::DdTh1520 => th1520::layout(artifacts, first_usable, last_usable),
        FlashVia::DdA733 => a733::layout(artifacts, first_usable, last_usable),
        other => Err(Error::InvalidPlan(format!(
            "partitions driver does not support `{other}`"
        ))),
    }
}

fn layout_root(artifacts: &[Artifact], first_usable: u64, last_usable: u64) -> Result<Layout> {
    let root = Artifact::by_role(artifacts, Role::Root)?;
    let mut cursor = first_usable;
    let p1 = allocate_rest(&mut cursor, sectors_for(root.size), last_usable, "root")?;

    Ok(Layout {
        partitions: vec![PartitionSpec::new("root", TYPE_LINUX_FS, p1)],
        writes: vec![WriteSpec::partition(p1, root)],
    })
}

fn layout_two_partitions(
    artifacts: &[Artifact],
    first_usable: u64,
    last_usable: u64,
    first_spec: TwoParts,
) -> Result<Layout> {
    let first = Artifact::by_role(artifacts, first_spec.role)?;
    let root = Artifact::by_role(artifacts, Role::Root)?;

    let mut cursor = first_usable;
    let first_sectors = first_spec.min_sectors.max(sectors_for(first.size));
    let p1 = allocate_fixed(&mut cursor, first_sectors, last_usable, first_spec.name)?;
    let p2 = allocate_rest(&mut cursor, sectors_for(root.size), last_usable, "root")?;

    Ok(Layout {
        partitions: vec![
            PartitionSpec::new(first_spec.name, first_spec.type_guid, p1),
            PartitionSpec::new("root", TYPE_LINUX_FS, p2),
        ],
        writes: vec![
            WriteSpec::partition(p1, first),
            WriteSpec::partition(p2, root),
        ],
    })
}

fn sectors_for(bytes: u64) -> u64 {
    bytes.div_ceil(SECTOR_SIZE).max(1)
}

fn allocate_fixed(
    cursor: &mut u64,
    sectors: u64,
    last_usable: u64,
    label: &str,
) -> Result<(u64, u64)> {
    let start = *cursor;
    let end = start
        .checked_add(sectors - 1)
        .ok_or_else(|| Error::InvalidPlan(format!("partition `{label}` size overflows")))?;
    if end > last_usable {
        return Err(Error::InvalidPlan(format!(
            "target is too small for partition `{label}`"
        )));
    }
    *cursor = end + 1;
    Ok((start, end))
}

fn allocate_rest(
    cursor: &mut u64,
    min_sectors: u64,
    last_usable: u64,
    label: &str,
) -> Result<(u64, u64)> {
    let start = *cursor;
    if start > last_usable || last_usable - start + 1 < min_sectors {
        return Err(Error::InvalidPlan(format!(
            "target is too small for partition `{label}`"
        )));
    }
    *cursor = last_usable + 1;
    Ok((start, last_usable))
}

fn random_guid() -> Result<[u8; 16]> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes)?;
    Ok(Builder::from_random_bytes(bytes).into_uuid().to_bytes_le())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::*;
    use crate::testutils::TestDir;

    #[tokio::test]
    async fn rejects_region_write_when_file_exceeds_allocated_limit() {
        let dir = TestDir::new("partitions", "oversized-region");
        let source_path = dir.join("root.img");
        let target_path = dir.join("disk.img");

        fs::write(&source_path, vec![0xABu8; 512]).unwrap();
        fs::write(&target_path, vec![0u8; 256 * 1024]).unwrap();

        let plan = FlashPlan {
            via: FlashVia::DdRoot,
            target: target_path.clone(),
            artifacts: vec![Artifact {
                path: source_path.clone(),
                role: Role::Root,
                size: 512,
            }],
        };

        let (file, mut writes) = partition_target(&plan).await.unwrap();
        writes[0].max_bytes = 512;
        fs::write(&source_path, vec![0xCDu8; 1024]).unwrap();

        let err = write_regions(Path::new(&target_path), file, &writes, &mut |_| {})
            .await
            .unwrap_err();
        assert!(matches!(err, Error::InvalidPlan(_)));
    }
}
