//! Partitioned block-device writes (`dd-boot-root`, `dd-efi-root`, `dd-root`, `dd-th1520`, `dd-a733`):
//! lay out a GPT partition table, then write each partition image (and any extra bootloader).
//!
//! | Method | Devices | Layout |
//! |---|---|---|
//! | `dd-boot-root` | eic7700 (SD, SATA, NVMe) | `p1` boot (>=2G) + `p2` root (rest) |
//! | `dd-efi-root` | qcom | `p1` EFI (>=512M) + `p2` root (rest) |
//! | `dd-root` | sg2042, sg2044, rockchip (SD, NVMe) | a single `root` partition |
//! | `dd-th1520` | th1520 (TF card) | `p1` 2M reserved + `p2` boot (>=500M) + `p3` 4G swap + `p4` root (rest) |
//! | `dd-a733` | a733 | `boot0` @ sector 256, `uboot` @ sector 24576, `p1` root @ sector 65536 |

mod a733;
mod boot_root;
mod efi_root;
mod root;
mod th1520;

use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use gptman::{GPT, GPTPartitionEntry};
use shared::catalog::{Artifact, FlashVia, Role};
use shared::fsx::{self, Amount, BUFFER_SIZE};
use shared::protocol::FlashPlan;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use uuid::{Builder, Uuid};

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

struct WriteSpec {
    sector: u64,
    path: PathBuf,
    size: u64,
}

struct Layout {
    partitions: Vec<PartitionSpec>,
    writes: Vec<WriteSpec>,
}

pub(super) async fn write(plan: &FlashPlan, mut progress: impl FnMut(Amount)) -> Result<()> {
    let (file, writes) = partition_target(plan).await?;
    write_regions(&plan.target, file, &writes, &mut progress).await
}

async fn partition_target(plan: &FlashPlan) -> Result<(std::fs::File, Vec<WriteSpec>)> {
    let target_path = plan.target.clone();
    let via = plan.via;
    let artifacts = plan.artifacts.clone();

    tokio::task::spawn_blocking(move || -> Result<(std::fs::File, Vec<WriteSpec>)> {
        let mut target = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&target_path)
            .map_err(|source| fsx::Error::write(&target_path, source))?;

        let disk_bytes = target
            .seek(SeekFrom::End(0))
            .map_err(|source| fsx::Error::write(&target_path, source))?;
        if disk_bytes < MIN_GPT_SECTORS * SECTOR_SIZE {
            return Err(Error::InvalidPlan(format!(
                "target `{}` ({disk_bytes} bytes) is too small for a GPT partition table",
                target_path.display()
            )));
        }

        GPT::write_protective_mbr_into(&mut target, SECTOR_SIZE)?;
        let mut gpt = GPT::new_from(&mut target, SECTOR_SIZE, random_guid()?)?;

        let layout = build_layout(
            via,
            &artifacts,
            gpt.header.first_usable_lba,
            gpt.header.last_usable_lba,
        )?;

        for (index, part) in layout.partitions.into_iter().enumerate() {
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
        target
            .flush()
            .map_err(|source| fsx::Error::write(&target_path, source))?;

        Ok((target, layout.writes))
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
    let mut target = File::from_std(file);
    let total: u64 = writes.iter().map(|spec| spec.size).sum();
    let mut done = 0u64;
    let mut buffer = vec![0u8; BUFFER_SIZE];

    for spec in writes {
        target
            .seek(SeekFrom::Start(spec.sector * SECTOR_SIZE))
            .await
            .map_err(|source| fsx::Error::write(target_path, source))?;

        let mut source = File::open(&spec.path)
            .await
            .map_err(|source| fsx::Error::read(&spec.path, source))?;

        loop {
            let read = source
                .read(&mut buffer)
                .await
                .map_err(|source| fsx::Error::read(&spec.path, source))?;
            if read == 0 {
                break;
            }

            target
                .write_all(&buffer[..read])
                .await
                .map_err(|source| fsx::Error::write(target_path, source))?;

            done += read as u64;
            progress(Amount::new(done, Some(total)));
        }
    }

    target
        .flush()
        .await
        .map_err(|source| fsx::Error::write(target_path, source))?;
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
        FlashVia::DdBootRoot => boot_root::layout(artifacts, first_usable, last_usable),
        FlashVia::DdEfiRoot => efi_root::layout(artifacts, first_usable, last_usable),
        FlashVia::DdRoot => root::layout(artifacts, first_usable, last_usable),
        FlashVia::DdTh1520 => th1520::layout(artifacts, first_usable, last_usable),
        FlashVia::DdA733 => a733::layout(artifacts, first_usable, last_usable),
        other => Err(Error::InvalidPlan(format!(
            "partitions driver does not support `{other}`"
        ))),
    }
}

fn layout_two_partitions(
    artifacts: &[Artifact],
    first_usable: u64,
    last_usable: u64,
    first_role: Role,
    first_name: &'static str,
    first_type: [u8; 16],
    first_min_sectors: u64,
) -> Result<Layout> {
    let first = Artifact::by_role(artifacts, first_role)?;
    let root = Artifact::by_role(artifacts, Role::Root)?;

    let mut cursor = first_usable;
    let first_sectors = first_min_sectors.max(sectors_for(first.size));
    let (p1_start, p1_end) = allocate_fixed(&mut cursor, first_sectors, last_usable, first_name)?;
    let (p2_start, p2_end) =
        allocate_rest(&mut cursor, sectors_for(root.size), last_usable, "root")?;

    Ok(Layout {
        partitions: vec![
            PartitionSpec {
                name: first_name,
                type_guid: first_type,
                starting_lba: p1_start,
                ending_lba: p1_end,
            },
            PartitionSpec {
                name: "root",
                type_guid: TYPE_LINUX_FS,
                starting_lba: p2_start,
                ending_lba: p2_end,
            },
        ],
        writes: vec![
            WriteSpec {
                sector: p1_start,
                path: first.path.clone(),
                size: first.size,
            },
            WriteSpec {
                sector: p2_start,
                path: root.path.clone(),
                size: root.size,
            },
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
