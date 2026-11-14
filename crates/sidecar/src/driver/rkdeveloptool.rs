//! `rkdeveloptool`: write the Rockchip eMMC from MaskROM mode.
//!
//! Devices: the rockchip group (Radxa Rock 5 series, Radxa Rock 4D, DshanPi A1
//! and other Rockchip boards) when the medium is the on-board eMMC. Their SD and
//! NVMe media are written through `dd-partitions` instead — the board only needs
//! MaskROM tooling when the medium cannot be reached as a block device.
//!
//! The board has to be in MaskROM mode (reset with the recovery button held),
//! which shows up as a Rockchip USB device rather than a disk. Writes address
//! the flash by sector offset, not by partition name:
//!
//! ```text
//! rkdeveloptool db <loader>          # download the loader into RAM
//! rkdeveloptool wl <offset> <image>  # write at a sector offset
//! ```
//!
//! The implementation will adapt `rkdeveloptool` the same way `fastboot` is
//! adapted, running as root:
//!
//! 1. find a board in MaskROM mode and load the loader that matches its SoC;
//! 2. write the root image ([`Role::Root`]) of `plan` at the offset the device's
//!    layout puts it at;
//! 3. reset the board and report [`Amount`] across the writes.
//!
//! Open questions: where the loader comes from (a published artifact of the board
//! or one bundled with the helper), and whether the offsets belong in
//! `devices.toml` next to the write method. The offsets must not come from the
//! client.
//!
//! [`Role::Root`]: catalog::Role::Root

use fsx::Amount;
use sidecar_protocol::flash::FlashPlan;

use crate::Result;

pub(super) fn write(_plan: &FlashPlan, _progress: &mut dyn FnMut(Amount)) -> Result<()> {
    todo!("write the Rockchip eMMC with rkdeveloptool")
}
