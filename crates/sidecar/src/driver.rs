//! The write drivers, one per `flash_via` method of the device index.
//!
//! `iso-write` is a separate method because a bootable ISO is its own kind of
//! artifact, but it writes like a whole-disk image and shares [`whole::write`].
//!
//! The only parts of the installer that touch a block device or USB fastboot
//! device, and they only run inside the privileged helper. `rkdeveloptool` is
//! not implemented yet (`todo!()`).

use shared::fsx::Amount;
use shared::protocol::FlashPlan;

use crate::Result;

mod fastboot;
mod partitions;
mod rkdeveloptool;
mod whole;

/// Write every artifact of `plan` with the driver for `plan.via`.
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
        shared::catalog::FlashVia::Rkdeveloptool => rkdeveloptool::write(plan, &mut progress).await,
    }
}
