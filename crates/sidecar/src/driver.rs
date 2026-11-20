//! The write drivers, one per `flash_via` method of the device index.
//!
//! `iso-write` is a separate method because a bootable ISO is its own kind of
//! artifact, but it writes like a whole-disk image and shares [`whole::write`].
//!
//! The only parts of the installer that touch a block device, USB fastboot, or
//! USB RockUSB device, and they only run inside the privileged helper.

use std::fmt::Display;
use std::path::Path;
use std::time::Duration;

use shared::fsx::{self, Amount};
use shared::protocol::FlashPlan;
use shared::targets::rockusb::DeviceInfo;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{Instant, sleep};

use crate::{Error, Result};

mod fastboot;
mod partitions;
mod rk;
mod whole;

const RECONNECT_SETTLE: Duration = Duration::from_secs(1);
const RECONNECT_POLL: Duration = Duration::from_millis(250);
const RECONNECT_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) struct Progress<'a, F: ?Sized> {
    done: u64,
    total: Option<u64>,
    last_reported: Option<u64>,
    report: &'a mut F,
}

impl<'a, F: FnMut(Amount) + ?Sized> Progress<'a, F> {
    pub(super) fn new(total: Option<u64>, report: &'a mut F) -> Self {
        Self {
            done: 0,
            total,
            last_reported: None,
            report,
        }
    }

    fn emit(&mut self, done: u64) {
        if self.last_reported != Some(done) {
            self.last_reported = Some(done);
            (self.report)(Amount::new(done, self.total));
        }
    }

    pub(super) fn report_offset(&mut self, current: u64) {
        self.emit(self.done + current);
    }

    pub(super) fn advance(&mut self, delta: u64) {
        self.done += delta;
        self.emit(self.done);
    }
}

pub(super) async fn copy(
    src: &mut File,
    src_path: &Path,
    dst: &mut File,
    dst_path: &Path,
    buf: &mut [u8],
    max_bytes: Option<u64>,
    mut on_chunk: impl FnMut(u64),
) -> Result<()> {
    let mut copied = 0u64;
    loop {
        let read = src
            .read(buf)
            .await
            .map_err(|source| fsx::Error::read(src_path, source))?;
        if read == 0 {
            return Ok(());
        }

        let next = copied
            .checked_add(read as u64)
            .ok_or_else(|| Error::InvalidPlan("write size overflows".into()))?;
        if let Some(limit) = max_bytes
            && next > limit
        {
            return Err(Error::InvalidPlan(format!(
                "`{}` exceeds allocated region limit ({limit} bytes)",
                src_path.display()
            )));
        }

        dst.write_all(&buf[..read])
            .await
            .map_err(|source| fsx::Error::write(dst_path, source))?;

        copied = next;
        on_chunk(read as u64);
    }
}

pub(super) async fn open_usb_device<D, I, E1, E2>(
    target: &Path,
    kind: &str,
    wrap_err: fn(String) -> Error,
    mut enumerate: impl AsyncFnMut() -> std::result::Result<I, E1>,
    mut open: impl AsyncFnMut(DeviceInfo) -> std::result::Result<D, E2>,
) -> Result<(D, DeviceInfo)>
where
    I: Iterator<Item = DeviceInfo>,
    E1: Display,
    E2: Display,
{
    let mut devices = enumerate()
        .await
        .map_err(|err| wrap_err(format!("failed to enumerate USB devices: {err}")))?;
    let info = devices
        .find(|info| shared::targets::matches(info, target))
        .ok_or_else(|| {
            wrap_err(format!(
                "no {kind} USB device matching `{}` found",
                target.display()
            ))
        })?;

    let device = open(info.clone())
        .await
        .map_err(|err| wrap_err(format!("failed to open {kind} USB device: {err}")))?;
    Ok((device, info))
}

pub(super) async fn reconnect_usb_device<D, I, E1, E2>(
    previous: &DeviceInfo,
    kind: &str,
    wrap_err: fn(String) -> Error,
    mut enumerate: impl AsyncFnMut() -> std::result::Result<I, E1>,
    mut open: impl AsyncFnMut(DeviceInfo) -> std::result::Result<D, E2>,
) -> Result<D>
where
    I: Iterator<Item = DeviceInfo>,
{
    sleep(RECONNECT_SETTLE).await;
    let deadline = Instant::now() + RECONNECT_TIMEOUT;

    loop {
        if let Ok(mut devices) = enumerate().await
            && let Some(info) = devices.find(|info| shared::targets::same_port(previous, info))
            && let Ok(device) = open(info).await
        {
            return Ok(device);
        }
        if Instant::now() >= deadline {
            return Err(wrap_err(format!(
                "timed out waiting for {kind} device to reconnect"
            )));
        }

        sleep(RECONNECT_POLL).await;
    }
}

pub(crate) async fn write(plan: &FlashPlan, mut progress: impl FnMut(Amount)) -> Result<()> {
    match plan.via {
        shared::catalog::FlashVia::DdWhole | shared::catalog::FlashVia::IsoWrite => {
            whole::write(plan, &mut progress).await
        }
        shared::catalog::FlashVia::DdBootRoot
        | shared::catalog::FlashVia::DdEfiRoot
        | shared::catalog::FlashVia::DdRoot
        | shared::catalog::FlashVia::DdTh1520
        | shared::catalog::FlashVia::DdA733 => partitions::write(plan, &mut progress).await,
        shared::catalog::FlashVia::Fastboot
        | shared::catalog::FlashVia::FastbootTh1520
        | shared::catalog::FlashVia::FastbootSpacemit => fastboot::write(plan, &mut progress).await,
        shared::catalog::FlashVia::RkLoader
        | shared::catalog::FlashVia::RkRoot
        | shared::catalog::FlashVia::RkSpi
        | shared::catalog::FlashVia::RkIdbloaderUboot => rk::write(plan, &mut progress).await,
    }
}
