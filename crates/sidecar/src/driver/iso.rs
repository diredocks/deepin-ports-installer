//! `iso-write`: write a bootable ISO to removable media.
//!
//! The generic group (QEMU, MilkV Titan, UltraRISC DP1000, Sophgo SG2044) ships one
//! `*.iso` that boots as it is. RISC-V boards cannot boot a Ventoy stick, so the
//! medium is written directly.

use shared::fsx::Amount;
use shared::protocol::flash::FlashPlan;

use crate::Result;

pub(super) fn write(_plan: &FlashPlan, _progress: &mut dyn FnMut(Amount)) -> Result<()> {
    todo!("write a bootable ISO to removable media")
}
