//! `fastboot`: flash named partitions through the bootloader over USB.
//!
//! Devices: eic7700 (eMMC), th1520, k1 and k3. The medium is inside the board —
//! an eMMC module or a soldered flash — and is only reachable after dropping into
//! u-boot's fastboot mode, usually from a serial console.
//!
//! What the bootloader accepts differs per family:
//!
//! * k1 and k3 need a partition table first, provided as a GPT JSON image that is
//!   flashed to a `gpt` partition; bootfs and rootfs follow as named partitions.
//! * th1520 on a TF card expects four partitions: a 2M header, `boot` (500M),
//!   `swap` (4G) and `root`.
//! * eic7700 flashes its boot and root images into partitions of the same names.
//!
//! The implementation will adapt the `fastboot` CLI rather than reimplement the
//! USB protocol, and only translate a [`FlashPlan`] into the right sequence of
//! `fastboot` invocations:
//!
//! 1. resolve the device in fastboot mode — by serial when several are plugged
//!    in — and fail instead of flashing the wrong board; the target the plan
//!    names is not a block device node, which is why the helper re-enumerates
//!    USB devices instead of trusting the path;
//! 2. flash the partition table when the layout needs one;
//! 3. flash each artifact to its partition name, reporting [`Amount`] across
//!    the partitions;
//! 4. optionally reboot the board when the client asks for it.

use fsx::Amount;
use sidecar_protocol::flash::FlashPlan;

use crate::Result;

pub(super) fn write(_plan: &FlashPlan, _progress: &mut dyn FnMut(Amount)) -> Result<()> {
    todo!("flash partitions through the fastboot CLI")
}
