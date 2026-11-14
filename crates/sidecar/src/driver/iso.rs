//! `iso-write`: write a bootable ISO to removable media.
//!
//! Devices: the generic group (QEMU, MilkV Titan, UltraRISC DP1000, Sophgo
//! SG2044). Their image is one `*.iso` ([`Role::Iso`]) that boots as it is, so
//! there is nothing to partition and nothing to unpack.
//!
//! RISC-V boards cannot boot a Ventoy stick, so the medium has to be written
//! directly; the ISO also stays bootable when it is simply copied to a file, which
//! is what the QEMU builds are used for.
//!
//! The implementation will:
//!
//! 1. take the single [`Role::Iso`] artifact and refuse a target that is not
//!    removable, so a system disk cannot be overwritten — the check has to come
//!    from the helper's own enumeration, not from the plan;
//! 2. stream the ISO to `plan.target`, reporting [`Amount`];
//! 3. leave the medium unmounted afterwards, the way an ISO image is expected to
//!    be.
//!
//! Whether a plain `dd` is enough or the ISO needs its hybrid boot records
//! rewritten is still open.
//!
//! [`Role::Iso`]: catalog::Role::Iso

use fsx::Amount;
use sidecar_protocol::flash::FlashPlan;

use crate::Result;

pub(super) fn write(_plan: &FlashPlan, _progress: &mut dyn FnMut(Amount)) -> Result<()> {
    todo!("write a bootable ISO to removable media")
}
