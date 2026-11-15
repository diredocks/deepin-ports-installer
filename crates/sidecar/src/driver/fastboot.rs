//! `fastboot`: flash named partitions through the bootloader over USB.
//!
//! eic7700 (eMMC), th1520, k1 and k3 keep the medium inside the board, reachable
//! only from u-boot's fastboot mode:
//!
//! * k1 and k3 need a GPT JSON image flashed to a `gpt` partition first;
//! * th1520 on a TF card expects a 2M header, `boot`, `swap` and `root`;
//! * eic7700 flashes boot and root into partitions of the same names.

use shared::fsx::Amount;
use shared::protocol::flash::FlashPlan;

use crate::Result;

pub(super) fn write(_plan: &FlashPlan, _progress: &mut dyn FnMut(Amount)) -> Result<()> {
    todo!("flash partitions through the fastboot CLI")
}
