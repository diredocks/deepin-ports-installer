//! `dd-partitions`: lay out a partition table, then write each partition image.
//!
//! eic7700 (SD, SATA, NVMe), sg2042, sg2044, q6a, rockchip (SD, NVMe) and a733
//! ship partition images rather than a whole disk.
//!
//! | Devices | Layout |
//! |---|---|
//! | eic7700 | `p1` boot (~2G) + `p2` root (rest) |
//! | q6a | `p1` EFI (>=256M) + `p2` root |
//! | sg2042, sg2044, rockchip | a single partition holding the root image |
//! | a733 | a single partition that must start at sector 65536 |

use shared::fsx::Amount;
use shared::protocol::flash::FlashPlan;

use crate::Result;

pub(super) fn write(_plan: &FlashPlan, _progress: &mut dyn FnMut(Amount)) -> Result<()> {
    todo!("partition the target and write partition images with dd")
}
