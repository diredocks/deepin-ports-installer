//! Partitioned block-device writes (`dd-boot-root`, `dd-efi-root`, `dd-root`, `dd-a733`):
//! lay out a partition table, then write each partition image (and any extra bootloader).
//!
//! | Method | Devices | Layout |
//! |---|---|---|
//! | `dd-boot-root` | eic7700 (SD, SATA, NVMe) | `p1` boot (~2G) + `p2` root (rest) |
//! | `dd-efi-root` | qcom | `p1` EFI (>=256M) + `p2` root |
//! | `dd-root` | sg2042, sg2044, rockchip (SD, NVMe) | a single partition holding the root image |
//! | `dd-a733` | a733 | U-Boot + a single `root` partition starting at sector 65536 |

use shared::fsx::Amount;
use shared::protocol::FlashPlan;

use crate::Result;

pub(super) async fn write(_plan: &FlashPlan, _progress: impl FnMut(Amount)) -> Result<()> {
    todo!("partition the target and write partition images with dd")
}
