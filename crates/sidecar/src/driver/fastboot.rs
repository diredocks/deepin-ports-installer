//! Fastboot drivers (`fastboot`, `fastboot-th1520`, `fastboot-spacemit`):
//! flash named partitions through the bootloader over USB.
//!
//! eic7700 (eMMC), th1520, k1 and k3 keep the medium inside the board, reachable
//! only from u-boot's fastboot mode:
//!
//! * `fastboot`: flashes `boot` and `root` into partitions of the same names (eic7700);
//! * `fastboot-th1520`: stages `uboot` to RAM, reboots, then flashes `uboot`, `boot` and `root` (th1520);
//! * `fastboot-spacemit`: flashes `gpt`, `bootinfo`, `fsbl`, `env`, `opensbi`, `uboot`, `boot` and `root` (k1, k3).

use shared::fsx::Amount;
use shared::protocol::FlashPlan;

use crate::Result;

pub(super) async fn write(_plan: &FlashPlan, _progress: impl FnMut(Amount)) -> Result<()> {
    todo!("flash partitions through the fastboot CLI")
}
