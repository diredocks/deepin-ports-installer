//! The write drivers, one per `flash_via` method of the device index.
//!
//! These are the only parts of the installer that touch a block device, and they
//! only ever run inside the privileged helper (see [`crate::serve`]): the
//! unprivileged frontend prepares the artifacts, packs them into a [`FlashPlan`],
//! and the helper hands it to [`write`].
//!
//! Only the method differs between the five. `dd-whole` writes a single
//! whole-disk image, `dd-partitions` lays out a partition table first,
//! `fastboot` and `rkdeveloptool` talk to a bootloader over USB instead of a
//! block device, and `iso-write` writes media a machine boots from as it is.
//!
//! None of them is implemented yet: each submodule documents the steps it will
//! take and ends in a `todo!()`, the same way the platform backends of `drives`
//! are scaffolded.

use fsx::Amount;
use sidecar_protocol::flash::FlashPlan;

use crate::Result;

mod fastboot;
mod iso;
mod partitions;
mod rkdeveloptool;
mod whole;

/// Write every artifact of `plan` with the driver for `plan.via`.
///
/// Progress is reported through `progress` as bytes are written, so the helper
/// can stream it to the client without knowing which driver is running.
pub(crate) fn write(plan: &FlashPlan, progress: &mut dyn FnMut(Amount)) -> Result<()> {
    match plan.via {
        catalog::FlashVia::DdWhole => whole::write(plan, progress),
        catalog::FlashVia::DdPartitions => partitions::write(plan, progress),
        catalog::FlashVia::Fastboot => fastboot::write(plan, progress),
        catalog::FlashVia::Rkdeveloptool => rkdeveloptool::write(plan, progress),
        catalog::FlashVia::IsoWrite => iso::write(plan, progress),
    }
}
