//! Fastboot drivers (`fastboot`, `fastboot-th1520`, `fastboot-spacemit`):
//! flash named partitions through the bootloader over USB using `fastboot-protocol`.

mod spacemit;

use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use android_sparse_image::split::{Split, SplitChunk, split_image, split_raw};
use android_sparse_image::{
    CHUNK_HEADER_BYTES_LEN, ChunkHeader, FILE_HEADER_BYTES_LEN, FileHeader, FileHeaderBytes,
    ParseError,
};
use fastboot_protocol::nusb::{DataDownload, DeviceInfo, NusbFastBoot, NusbFastBootError};
use fastboot_protocol::protocol::parse_u32;
use shared::catalog::{Artifact, FlashVia, Role};
use shared::fsx::{self, Amount};
use shared::protocol::FlashPlan;
use tokio::fs::File;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeekExt};

use super::{Progress, open_usb_device, reconnect_usb_device};
use crate::{Error, Result};

const DEFAULT_MAX_DOWNLOAD: u32 = 256 * 1024 * 1024;

enum Step {
    StageAndContinue {
        path: PathBuf,
        size: u64,
    },
    Flash {
        partition: &'static str,
        path: PathBuf,
        size: u64,
    },
    FlashBytes {
        partition: &'static str,
        data: &'static [u8],
    },
    RebootAndReconnect,
    Reboot,
}

pub(super) async fn write(plan: &FlashPlan, mut progress: impl FnMut(Amount)) -> Result<()> {
    let steps = build_steps(plan.via, &plan.artifacts)?;
    execute_steps(&plan.target, &steps, &mut progress).await
}

fn build_steps(via: FlashVia, artifacts: &[Artifact]) -> Result<Vec<Step>> {
    match via {
        FlashVia::Fastboot => Ok(vec![
            flash_step(artifacts, "boot", Role::Boot)?,
            flash_step(artifacts, "root", Role::Root)?,
        ]),
        FlashVia::FastbootTh1520 => Ok(vec![
            flash_step(artifacts, "ram", Role::Uboot)?,
            Step::RebootAndReconnect,
            flash_step(artifacts, "uboot", Role::Uboot)?,
            flash_step(artifacts, "boot", Role::Boot)?,
            flash_step(artifacts, "root", Role::Root)?,
            Step::Reboot,
        ]),
        FlashVia::FastbootSpacemit => spacemit::steps(artifacts),
        other => Err(Error::InvalidPlan(format!(
            "fastboot driver does not support `{other}`"
        ))),
    }
}

async fn execute_steps(
    target: &Path,
    steps: &[Step],
    progress: &mut impl FnMut(Amount),
) -> Result<()> {
    let total: u64 = steps
        .iter()
        .map(|step| match step {
            Step::StageAndContinue { size, .. } | Step::Flash { size, .. } => *size,
            Step::FlashBytes { data, .. } => data.len() as u64,
            Step::RebootAndReconnect | Step::Reboot => 0,
        })
        .sum();

    let (mut fb, previous) = open_usb_device(
        target,
        "fastboot",
        Error::Fastboot,
        fastboot_protocol::nusb::devices,
        |info| async move { NusbFastBoot::from_info(&info).await },
    )
    .await?;
    let mut progress = Progress::new(Some(total), progress);
    let mut max_download: Option<u32> = None;

    for step in steps {
        match step {
            Step::StageAndContinue { path, size } => {
                stage_file(&mut fb, path, *size, &mut progress).await?;
                progress.advance(*size);

                fb.continue_boot()
                    .await
                    .map_err(|err| Error::Fastboot(format!("continue boot failed: {err}")))?;
                drop(fb);
                fb = reconnect(&previous).await?;
                max_download = None;
            }
            Step::Flash {
                partition,
                path,
                size,
            } => {
                flash_file(
                    &mut fb,
                    partition,
                    path,
                    *size,
                    &mut max_download,
                    &mut progress,
                )
                .await?;
                progress.advance(*size);
            }
            Step::FlashBytes { partition, data } => {
                flash_bytes(&mut fb, partition, data, &mut progress).await?;
                progress.advance(data.len() as u64);
            }
            Step::RebootAndReconnect => {
                fb.reboot()
                    .await
                    .map_err(|err| Error::Fastboot(format!("reboot failed: {err}")))?;
                drop(fb);
                fb = reconnect(&previous).await?;
                max_download = None;
            }
            Step::Reboot => {
                fb.reboot()
                    .await
                    .map_err(|err| Error::Fastboot(format!("reboot failed: {err}")))?;
            }
        }
    }

    Ok(())
}

fn stage_step(artifacts: &[Artifact], role: Role) -> Result<Step> {
    let artifact = Artifact::by_role(artifacts, role)?;
    Ok(Step::StageAndContinue {
        path: artifact.path.clone(),
        size: artifact.size,
    })
}

fn flash_step(artifacts: &[Artifact], partition: &'static str, role: Role) -> Result<Step> {
    let artifact = Artifact::by_role(artifacts, role)?;
    Ok(Step::Flash {
        partition,
        path: artifact.path.clone(),
        size: artifact.size,
    })
}

async fn reconnect(previous: &DeviceInfo) -> Result<NusbFastBoot> {
    reconnect_usb_device(
        previous,
        "fastboot",
        Error::Fastboot,
        fastboot_protocol::nusb::devices,
        |info| async move { NusbFastBoot::from_info(&info).await },
    )
    .await
}

fn checked_download_size(path: &Path, file_size: u64) -> Result<u32> {
    u32::try_from(file_size).map_err(|_| {
        Error::InvalidPlan(format!(
            "`{}` ({file_size} bytes) exceeds 4 GiB fastboot download limit",
            path.display()
        ))
    })
}

async fn stage_file<F: FnMut(Amount) + ?Sized>(
    fb: &mut NusbFastBoot,
    path: &Path,
    file_size: u64,
    progress: &mut Progress<'_, F>,
) -> Result<()> {
    let size = checked_download_size(path, file_size)?;
    let mut file = File::open(path)
        .await
        .map_err(|source| fsx::Error::read(path, source))?;

    download_raw(fb, path, &mut file, size, progress).await
}

async fn flash_file<F: FnMut(Amount) + ?Sized>(
    fb: &mut NusbFastBoot,
    partition: &str,
    path: &Path,
    file_size: u64,
    cached_max_download: &mut Option<u32>,
    progress: &mut Progress<'_, F>,
) -> Result<()> {
    let max_download = match *cached_max_download {
        Some(size) => size,
        None => {
            let size = query_max_download(fb).await?;
            *cached_max_download = Some(size);
            size
        }
    };

    let splits = prepare_splits(path, file_size, max_download).await?;
    let mut file = File::open(path)
        .await
        .map_err(|source| fsx::Error::read(path, source))?;

    if let Some(splits) = splits {
        return flash_splits(fb, partition, path, &mut file, file_size, &splits, progress).await;
    }

    let size = checked_download_size(path, file_size)?;
    download_raw(fb, path, &mut file, size, progress).await?;
    fb.flash(partition)
        .await
        .map_err(|err| Error::Fastboot(format!("failed to flash `{partition}`: {err}")))
}

async fn flash_bytes<F: FnMut(Amount) + ?Sized>(
    fb: &mut NusbFastBoot,
    partition: &str,
    data: &[u8],
    progress: &mut Progress<'_, F>,
) -> Result<()> {
    let size = u32::try_from(data.len()).map_err(|_| {
        Error::InvalidPlan(format!(
            "inline payload for `{partition}` ({} bytes) exceeds 4 GiB fastboot download limit",
            data.len()
        ))
    })?;

    download_raw(fb, Path::new("<inline>"), &mut &*data, size, progress).await?;
    fb.flash(partition)
        .await
        .map_err(|err| Error::Fastboot(format!("failed to flash `{partition}`: {err}")))
}

async fn query_max_download(fb: &mut NusbFastBoot) -> Result<u32> {
    let value = match fb.get_var("max-download-size").await {
        Ok(value) => value,
        Err(NusbFastBootError::FastbootFailed(_)) => return Ok(DEFAULT_MAX_DOWNLOAD),
        Err(err) => {
            return Err(Error::Fastboot(format!(
                "failed to query `max-download-size`: {err}"
            )));
        }
    };

    Ok(parse_u32(value.trim())
        .ok()
        .filter(|&size| size > 0)
        .unwrap_or(DEFAULT_MAX_DOWNLOAD))
}

async fn prepare_splits(
    path: &Path,
    file_size: u64,
    max_download: u32,
) -> Result<Option<Vec<Split>>> {
    if let Some(splits) = try_parse_sparse_splits(path, file_size, max_download).await? {
        return Ok(Some(splits));
    }
    if file_size <= u64::from(max_download) {
        return Ok(None);
    }

    let splits = split_raw(file_size, u64::from(max_download)).map_err(|err| {
        Error::Fastboot(format!(
            "failed to split raw image `{}`: {err}",
            path.display()
        ))
    })?;
    Ok(Some(splits))
}

async fn try_parse_sparse_splits(
    path: &Path,
    file_size: u64,
    max_download: u32,
) -> Result<Option<Vec<Split>>> {
    if file_size < FILE_HEADER_BYTES_LEN as u64 {
        return Ok(None);
    }

    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || parse_sparse_splits_sync(&path, max_download))
        .await
        .map_err(|err| Error::Io(std::io::Error::other(err)))?
}

fn parse_sparse_splits_sync(path: &Path, max_download: u32) -> Result<Option<Vec<Split>>> {
    let mut file = std::fs::File::open(path).map_err(|source| fsx::Error::read(path, source))?;

    let mut header_bytes = FileHeaderBytes::default();
    file.read_exact(&mut header_bytes)
        .map_err(|source| fsx::Error::read(path, source))?;

    let header = match FileHeader::from_bytes(&header_bytes) {
        Ok(header) => header,
        Err(ParseError::UnknownMagic) => return Ok(None),
        Err(err) => {
            return Err(Error::InvalidPlan(format!(
                "invalid sparse header in `{}`: {err}",
                path.display()
            )));
        }
    };

    let mut chunks = Vec::with_capacity(header.chunks as usize);
    for _ in 0..header.chunks {
        let mut chunk_bytes = [0u8; CHUNK_HEADER_BYTES_LEN];
        file.read_exact(&mut chunk_bytes)
            .map_err(|source| fsx::Error::read(path, source))?;
        let chunk = ChunkHeader::from_bytes(&chunk_bytes).map_err(|err| {
            Error::InvalidPlan(format!(
                "invalid sparse chunk in `{}`: {err}",
                path.display()
            ))
        })?;
        let data_size = i64::try_from(chunk.data_size()).map_err(|_| {
            Error::InvalidPlan(format!(
                "sparse chunk size overflow in `{}`",
                path.display()
            ))
        })?;
        file.seek(SeekFrom::Current(data_size))
            .map_err(|source| fsx::Error::read(path, source))?;
        chunks.push(chunk);
    }

    let splits = split_image(&header, &chunks, u64::from(max_download)).map_err(|err| {
        Error::Fastboot(format!(
            "failed to split sparse image `{}`: {err}",
            path.display()
        ))
    })?;
    Ok(Some(splits))
}

async fn download_raw<R: AsyncRead + Unpin, F: FnMut(Amount) + ?Sized>(
    fb: &mut NusbFastBoot,
    path: &Path,
    input: &mut R,
    size: u32,
    progress: &mut Progress<'_, F>,
) -> Result<()> {
    let mut sender = fb
        .download(size)
        .await
        .map_err(|err| Error::Fastboot(format!("fastboot download failed: {err}")))?;

    let mut sent = 0u64;
    while sender.left() > 0 {
        let left = sender.left() as usize;
        let buf = sender
            .get_mut_data(left)
            .await
            .map_err(|err| Error::Fastboot(format!("fastboot transfer failed: {err}")))?;
        input
            .read_exact(buf)
            .await
            .map_err(|source| fsx::Error::read(path, source))?;
        sent += buf.len() as u64;
        progress.report_offset(sent);
    }

    sender
        .finish()
        .await
        .map_err(|err| Error::Fastboot(format!("fastboot download finish failed: {err}")))
}

async fn flash_splits<F: FnMut(Amount) + ?Sized>(
    fb: &mut NusbFastBoot,
    partition: &str,
    path: &Path,
    file: &mut File,
    file_size: u64,
    splits: &[Split],
    progress: &mut Progress<'_, F>,
) -> Result<()> {
    let mut step_read = 0u64;

    for split in splits {
        send_sparse_split(fb, path, file, file_size, split, &mut step_read, progress).await?;
        fb.flash(partition)
            .await
            .map_err(|err| Error::Fastboot(format!("failed to flash `{partition}`: {err}")))?;
    }

    Ok(())
}

async fn send_sparse_split<F: FnMut(Amount) + ?Sized>(
    fb: &mut NusbFastBoot,
    path: &Path,
    file: &mut File,
    file_size: u64,
    split: &Split,
    step_read: &mut u64,
    progress: &mut Progress<'_, F>,
) -> Result<()> {
    let sparse_size = u32::try_from(split.sparse_size()).map_err(|_| {
        Error::Fastboot(format!(
            "sparse split size ({}) exceeds 4 GiB",
            split.sparse_size()
        ))
    })?;

    let mut sender = fb
        .download(sparse_size)
        .await
        .map_err(|err| Error::Fastboot(format!("fastboot download failed: {err}")))?;

    sender
        .extend_from_slice(&split.header.to_bytes())
        .await
        .map_err(|err| Error::Fastboot(format!("fastboot header write failed: {err}")))?;

    for chunk in &split.chunks {
        sender
            .extend_from_slice(&chunk.header.to_bytes())
            .await
            .map_err(|err| Error::Fastboot(format!("fastboot chunk write failed: {err}")))?;

        if chunk.size == 0 {
            continue;
        }

        stream_chunk_payload(
            &mut sender,
            path,
            file,
            file_size,
            chunk,
            step_read,
            progress,
        )
        .await?;
    }

    sender
        .finish()
        .await
        .map_err(|err| Error::Fastboot(format!("fastboot download finish failed: {err}")))
}

async fn stream_chunk_payload<F: FnMut(Amount) + ?Sized>(
    sender: &mut DataDownload<'_>,
    path: &Path,
    file: &mut File,
    file_size: u64,
    chunk: &SplitChunk,
    step_read: &mut u64,
    progress: &mut Progress<'_, F>,
) -> Result<()> {
    file.seek(SeekFrom::Start(chunk.offset))
        .await
        .map_err(|source| fsx::Error::read(path, source))?;

    let mut left = chunk.size;
    while left > 0 {
        let want = usize::try_from(left).unwrap_or(usize::MAX);
        let buf = sender
            .get_mut_data(want)
            .await
            .map_err(|err| Error::Fastboot(format!("fastboot transfer failed: {err}")))?;
        let filled = buf.len() as u64;
        read_exact_padded(file, buf)
            .await
            .map_err(|source| fsx::Error::read(path, source))?;
        left -= filled;
        *step_read = (*step_read + filled).min(file_size);
        progress.report_offset(*step_read);
    }

    Ok(())
}

async fn read_exact_padded<R: AsyncRead + Unpin>(
    input: &mut R,
    buf: &mut [u8],
) -> std::io::Result<()> {
    let total = buf.len();
    let mut offset = 0;
    while offset < total {
        match input.read(&mut buf[offset..]).await {
            Ok(0) => {
                buf[offset..].fill(0);
                break;
            }
            Ok(read) => offset += read,
            Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(err),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::testutils::TestDir;

    #[tokio::test]
    async fn prepare_splits_respects_exact_max_download_boundary() {
        let dir = TestDir::new("fastboot", "split-boundary");
        let raw_path = dir.join("raw.bin");
        fs::write(&raw_path, vec![0x11u8; 16384]).unwrap();

        assert!(
            prepare_splits(&raw_path, 16384, 16384)
                .await
                .unwrap()
                .is_none()
        );

        let splits = prepare_splits(&raw_path, 16384, 8192)
            .await
            .unwrap()
            .unwrap();
        assert!(splits.len() >= 2);
    }
}
