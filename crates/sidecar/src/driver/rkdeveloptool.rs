//! `rkdeveloptool`: download a Rockchip MaskROM loader into RAM and write
//! `root` (`0x40000`) or bootloader (`0` for SPI, or `0x40` + `0x4000`) over RockUSB.

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::time::Duration;

use rockfile::boot::{
    RkBootEntry, RkBootEntryBytes, RkBootHeader, RkBootHeaderBytes, RkBootHeaderEntry,
};
use rockusb::device::{DeviceAsync, TransportAsync};
use rockusb::protocol::ResetOpcode;
use shared::catalog::{Artifact, FlashVia, Role};
use shared::fsx::{self, Amount};
use shared::protocol::FlashPlan;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::time::sleep;

use super::{Progress, open_usb_device, reconnect_usb_device};
use crate::{Error, Result};

const CODE_471: u16 = 0x471;
const CODE_472: u16 = 0x472;
const SPI_UBOOT_SECTOR: u32 = 0;
const IDBLOADER_SECTOR: u32 = 0x40;
const UBOOT_SECTOR: u32 = 0x4000;
const ROOT_SECTOR: u32 = 0x40000;
const SECTOR_SIZE: usize = rockusb::protocol::SECTOR_SIZE as usize;
const MAX_WRITE_SECTORS: usize = 128;
const WRITE_CHUNK_SIZE: usize = MAX_WRITE_SECTORS * SECTOR_SIZE;

#[derive(Debug, Clone, PartialEq, Eq)]
struct BootBlob {
    area: u16,
    data: Vec<u8>,
    delay: Duration,
}

fn build_writes(plan: &FlashPlan) -> Result<Vec<(u32, &Artifact, Option<u64>)>> {
    match plan.via {
        FlashVia::Rkdeveloptool => Ok(vec![(
            ROOT_SECTOR,
            Artifact::by_role(&plan.artifacts, Role::Root)?,
            None,
        )]),
        FlashVia::RkdeveloptoolSpi => Ok(vec![(
            SPI_UBOOT_SECTOR,
            Artifact::by_role(&plan.artifacts, Role::Uboot)?,
            None,
        )]),
        FlashVia::RkdeveloptoolIdbloaderUboot => {
            let max_idbloader = u64::from(UBOOT_SECTOR - IDBLOADER_SECTOR) * SECTOR_SIZE as u64;
            Ok(vec![
                (
                    IDBLOADER_SECTOR,
                    Artifact::by_role(&plan.artifacts, Role::Idbloader)?,
                    Some(max_idbloader),
                ),
                (
                    UBOOT_SECTOR,
                    Artifact::by_role(&plan.artifacts, Role::Uboot)?,
                    None,
                ),
            ])
        }
        other => Err(Error::InvalidPlan(format!(
            "rkdeveloptool driver does not support `{other}`"
        ))),
    }
}

pub(super) async fn write(plan: &FlashPlan, mut progress: impl FnMut(Amount)) -> Result<()> {
    let specs = build_writes(plan)?;
    let loader = Artifact::by_role(&plan.artifacts, Role::Loader)?;
    let blobs = parse_loader(&loader.path)?;

    let mut writes = Vec::with_capacity(specs.len());
    let mut lba_bytes = 0u64;
    for (sector, artifact, max_bytes) in specs {
        let file = File::open(&artifact.path)
            .await
            .map_err(|source| fsx::Error::read(&artifact.path, source))?;
        let size = file
            .metadata()
            .await
            .map_err(|source| fsx::Error::read(&artifact.path, source))?
            .len();

        if let Some(limit) = max_bytes
            && size > limit
        {
            return Err(Error::InvalidPlan(format!(
                "`{}` ({size} bytes) exceeds allocated region limit ({limit} bytes)",
                artifact.path.display()
            )));
        }
        lba_bytes = lba_bytes
            .checked_add(size)
            .ok_or_else(|| Error::InvalidPlan("total write size overflows".into()))?;
        writes.push((sector, &artifact.path, file));
    }

    let loader_bytes: u64 = blobs.iter().map(|blob| blob.data.len() as u64).sum();
    let total = loader_bytes
        .checked_add(lba_bytes)
        .ok_or_else(|| Error::InvalidPlan("total write size overflows".into()))?;

    let (mut device, previous) = open_usb_device(
        &plan.target,
        "rockusb",
        Error::Rockusb,
        rockusb::nusb::devices,
        rockusb::nusb::Device::from_usb_device_info,
    )
    .await?;
    let mut progress = Progress::new(Some(total), &mut progress);

    download_boot(&mut device, &blobs, &mut progress).await?;
    drop(device);

    let mut device = reconnect_usb_device(
        &previous,
        "rockusb",
        Error::Rockusb,
        rockusb::nusb::devices,
        rockusb::nusb::Device::from_usb_device_info,
    )
    .await?;

    let mut buffer = vec![0u8; WRITE_CHUNK_SIZE];
    for (sector, path, mut file) in writes {
        write_lba_file(&mut device, path, &mut file, sector, &mut buffer, &mut progress).await?;
    }

    device
        .reset_device(ResetOpcode::Reset)
        .await
        .map_err(|err| Error::Rockusb(format!("failed to reset device: {err}")))
}

fn parse_loader(path: &Path) -> Result<Vec<BootBlob>> {
    let invalid = |msg: &str| {
        Error::InvalidPlan(format!(
            "invalid Rockchip loader `{}`: {msg}",
            path.display()
        ))
    };
    let map_io = |err: std::io::Error| match err.kind() {
        std::io::ErrorKind::InvalidData | std::io::ErrorKind::UnexpectedEof => {
            invalid(&err.to_string())
        }
        _ => fsx::Error::read(path, err).into(),
    };

    let mut file = std::fs::File::open(path).map_err(map_io)?;
    let mut header: RkBootHeaderBytes = [0; size_of::<RkBootHeaderBytes>()];
    file.read_exact(&mut header).map_err(map_io)?;

    let header = RkBootHeader::from_bytes(&header).ok_or_else(|| invalid("invalid header"))?;

    let mut blobs = parse_entry(header.entry_471, CODE_471, &mut file).map_err(map_io)?;
    blobs.extend(parse_entry(header.entry_472, CODE_472, &mut file).map_err(map_io)?);

    if blobs.is_empty() {
        return Err(invalid("contains no MaskROM boot entries"));
    }

    Ok(blobs)
}

fn parse_entry(
    header: RkBootHeaderEntry,
    area: u16,
    file: &mut std::fs::File,
) -> std::io::Result<Vec<BootBlob>> {
    if header.count > 0 && usize::from(header.size) < size_of::<RkBootEntryBytes>() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("invalid boot entry size ({})", header.size),
        ));
    }

    let mut blobs = Vec::with_capacity(usize::from(header.count));
    for i in 0..header.count {
        let mut entry: RkBootEntryBytes = [0; size_of::<RkBootEntryBytes>()];
        file.seek(SeekFrom::Start(
            u64::from(header.offset) + u64::from(header.size) * u64::from(i),
        ))?;
        file.read_exact(&mut entry)?;
        let entry = RkBootEntry::from_bytes(&entry);

        file.seek(SeekFrom::Start(u64::from(entry.data_offset)))?;
        let mut data = Vec::new();
        file.by_ref()
            .take(u64::from(entry.data_size))
            .read_to_end(&mut data)?;
        if data.len() != entry.data_size as usize {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "truncated boot payload",
            ));
        }

        blobs.push(BootBlob {
            area,
            data,
            delay: Duration::from_millis(u64::from(entry.data_delay)),
        });
    }

    Ok(blobs)
}

async fn download_boot<T, F>(
    device: &mut DeviceAsync<T>,
    blobs: &[BootBlob],
    progress: &mut Progress<'_, F>,
) -> Result<()>
where
    T: TransportAsync,
    F: FnMut(Amount) + ?Sized,
{
    for blob in blobs {
        device
            .write_maskrom_area(blob.area, &blob.data)
            .await
            .map_err(|err| {
                Error::Rockusb(format!(
                    "failed to write maskrom area {:#x}: {err}",
                    blob.area
                ))
            })?;
        progress.advance(blob.data.len() as u64);

        if !blob.delay.is_zero() {
            sleep(blob.delay).await;
        }
    }

    Ok(())
}

async fn write_lba_file<T, F>(
    device: &mut DeviceAsync<T>,
    path: &Path,
    file: &mut File,
    mut sector: u32,
    buffer: &mut [u8],
    progress: &mut Progress<'_, F>,
) -> Result<()>
where
    T: TransportAsync,
    F: FnMut(Amount) + ?Sized,
{
    loop {
        let read = read_chunk(file, path, buffer).await?;
        if read == 0 {
            break;
        }

        let padded = read.next_multiple_of(SECTOR_SIZE);
        if padded > read {
            buffer[read..padded].fill(0);
        }

        let sectors = u32::try_from(padded / SECTOR_SIZE).expect("chunk sectors fit in u32");
        device
            .write_lba(sector, &buffer[..padded])
            .await
            .map_err(|err| {
                Error::Rockusb(format!("failed to write LBA at sector {sector:#x}: {err}"))
            })?;

        sector = sector
            .checked_add(sectors)
            .ok_or_else(|| Error::InvalidPlan("LBA sector offset overflow".into()))?;
        progress.advance(read as u64);
    }

    Ok(())
}

async fn read_chunk(file: &mut File, path: &Path, buf: &mut [u8]) -> Result<usize> {
    let mut offset = 0;
    while offset < buf.len() {
        let read = file
            .read(&mut buf[offset..])
            .await
            .map_err(|source| fsx::Error::read(path, source))?;
        if read == 0 {
            break;
        }
        offset += read;
    }
    Ok(offset)
}
