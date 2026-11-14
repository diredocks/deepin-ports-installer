//! `dd-whole`: write a single whole-disk image, as `dd if=<img> of=/dev/sdX`.
//!
//! jh7110 (VisionFive 2, MilkV Mars, DC ROMA) and phytiumpi ship one `*.img` that
//! already carries the partition table and the bootloader.

use fsx::Amount;
use sidecar_protocol::flash::FlashPlan;

use crate::Result;

pub(super) fn write(_plan: &FlashPlan, _progress: &mut dyn FnMut(Amount)) -> Result<()> {
    todo!("write a whole-disk image with dd")
}
