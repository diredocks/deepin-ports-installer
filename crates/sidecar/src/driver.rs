//! The write drivers, one per `flash_via` method of the device index.
//!
//! The only parts of the installer that touch a block device, and they only run
//! inside the privileged helper. None is implemented yet (`todo!()`).

use fsx::Amount;
use sidecar_protocol::flash::FlashPlan;

use crate::Result;

mod fastboot;
mod iso;
mod partitions;
mod rkdeveloptool;
mod whole;

/// Write every artifact of `plan` with the driver for `plan.via`.
pub(crate) fn write(plan: &FlashPlan, progress: &mut dyn FnMut(Amount)) -> Result<()> {
    match plan.via {
        catalog::FlashVia::DdWhole => whole::write(plan, progress),
        catalog::FlashVia::DdPartitions => partitions::write(plan, progress),
        catalog::FlashVia::Fastboot => fastboot::write(plan, progress),
        catalog::FlashVia::Rkdeveloptool => rkdeveloptool::write(plan, progress),
        catalog::FlashVia::IsoWrite => iso::write(plan, progress),
    }
}
