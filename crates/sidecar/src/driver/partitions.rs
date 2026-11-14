//! `dd-partitions`: lay out a partition table, then write each partition image.
//!
//! Devices: eic7700 (SD, SATA, NVMe), sg2042, sg2044, q6a, rockchip (SD, NVMe)
//! and a733. The archive holds partition images rather than a whole disk, so the
//! table the images expect has to be created first and each image written where
//! its partition lives.
//!
//! The layouts differ per device, which is what the configuration is for:
//!
//! | Devices | Layout |
//! |---|---|
//! | eic7700 | `p1` boot (~2G) + `p2` root (rest) |
//! | q6a | `p1` EFI (>=256M) + `p2` root |
//! | sg2042, sg2044, rockchip | a single partition holding the root image |
//! | a733 | a single partition that must start at sector 65536 |
//!
//! Two quirks are already known: q6a boots from UFS and needs 4Kn sector
//! support, and a733's bootloader overwrites the first sectors of the medium, so
//! its partition table has to leave them empty.
//!
//! The implementation will:
//!
//! 1. turn the artifact roles ([`Role::Boot`], [`Role::Efi`], [`Role::Root`]) of
//!    `plan` into the layout of the device group, refusing a combination the
//!    group does not describe;
//! 2. write the table — and its alignment, sector size and reserved sectors —
//!    then wait for the kernel to re-read it;
//! 3. write each partition image in offset order, reporting [`Amount`], and
//!    flush the medium at the end.
//!
//! Creating the table itself is the open question: an in-process GPT writer keeps
//! the helper free of external tools but has to know every layout detail, while
//! shelling out to `sfdisk`/`sgdisk` inherits the same dependencies the official
//! guides already rely on. Either way the layout comes from the device index the
//! helper carries, never from the client.
//!
//! [`Role::Boot`]: catalog::Role::Boot
//! [`Role::Efi`]: catalog::Role::Efi
//! [`Role::Root`]: catalog::Role::Root

use fsx::Amount;
use sidecar_protocol::flash::FlashPlan;

use crate::Result;

pub(super) fn write(_plan: &FlashPlan, _progress: &mut dyn FnMut(Amount)) -> Result<()> {
    todo!("partition the target and write partition images with dd")
}
