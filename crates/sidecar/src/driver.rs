//! The write drivers, one per `flash_via` method of the device index.
//!
//! The only parts of the installer that touch a block device, and they only run
//! inside the privileged helper. None is implemented yet (`todo!()`).

use shared::fsx::Amount;
use shared::protocol::FlashPlan;

use crate::Result;

mod fastboot;
mod iso;
mod partitions;
mod rkdeveloptool;
mod whole;

/// Write every artifact of `plan` with the driver for `plan.via`.
pub(crate) async fn write(plan: &FlashPlan, mut progress: impl FnMut(Amount)) -> Result<()> {
    match plan.via {
        shared::catalog::FlashVia::DdWhole => whole::write(plan, &mut progress).await,
        shared::catalog::FlashVia::DdPartitions => partitions::write(plan, &mut progress).await,
        shared::catalog::FlashVia::Fastboot => fastboot::write(plan, &mut progress).await,
        shared::catalog::FlashVia::Rkdeveloptool => rkdeveloptool::write(plan, &mut progress).await,
        shared::catalog::FlashVia::IsoWrite => iso::write(plan, &mut progress).await,
    }
}
