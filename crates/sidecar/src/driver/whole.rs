//! `dd-whole`: write a single whole-disk image, as `dd if=<img> of=/dev/sdX`.
//!
//! Devices: jh7110 (VisionFive 2, MilkV Mars, DC ROMA) and phytiumpi. Their
//! archives hold one `*.img` ([`Role::Disk`]) that already carries the partition
//! table and the bootloader, so the medium is used as it comes — no partitioning,
//! no per-partition writes.
//!
//! The implementation will:
//!
//! 1. take the single [`Role::Disk`] artifact and refuse anything else, so a
//!    mismatched configuration cannot silently write the wrong medium;
//! 2. bail out when the target is smaller than the image;
//! 3. stream the image to `plan.target`, reporting [`Amount`], and flush plus
//!    re-read the medium before claiming success.
//!
//! phytiumpi is flashed the same way but publishes `b2sum` checksums; that
//! difference belongs to the unprivileged side, not here.
//!
//! [`Role::Disk`]: catalog::Role::Disk

use fsx::Amount;
use sidecar_protocol::flash::FlashPlan;

use crate::Result;

pub(super) fn write(_plan: &FlashPlan, _progress: &mut dyn FnMut(Amount)) -> Result<()> {
    todo!("write a whole-disk image with dd")
}
