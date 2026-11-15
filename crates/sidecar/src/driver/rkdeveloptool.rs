//! `rkdeveloptool`: write the Rockchip eMMC from MaskROM mode, which addresses the
//! flash by sector offset rather than by partition name.
//!
//! ```text
//! rkdeveloptool db <loader>          # download the loader into RAM
//! rkdeveloptool wl <offset> <image>  # write at a sector offset
//! ```
//!
//! The rockchip group (Radxa Rock 5 series, Rock 4D, DshanPi A1) needs this only
//! for the on-board eMMC; its SD and NVMe media go through `dd-partitions`.

use shared::fsx::Amount;
use shared::protocol::flash::FlashPlan;

use crate::Result;

pub(super) fn write(_plan: &FlashPlan, _progress: &mut dyn FnMut(Amount)) -> Result<()> {
    todo!("write the Rockchip eMMC with rkdeveloptool")
}
