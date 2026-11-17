use std::fmt;

use serde::{Deserialize, Serialize};

use crate::catalog::Role;

/// How an image is written to a medium, as named by `flash_via` in `devices.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
#[serde(rename_all = "kebab-case")]
pub enum FlashVia {
    /// Write a single whole-disk image.
    DdWhole,

    /// Write a bootable ISO to removable media as a whole disk image.
    IsoWrite,

    /// Partition the medium (`boot` + `root`), then write each partition image in place.
    DdBootRoot,

    /// Partition the medium (`efi` + `root`), then write each partition image in place.
    DdEfiRoot,

    /// Create a single `root` partition on the medium and write the root image in place.
    DdRoot,

    /// Partition the T-Head TH1520 TF card (2M reserved + `boot` + `swap` + `root`) and write `boot` and `root`.
    DdTh1520,

    /// Partition the Allwinner A733 medium (`root` starting at sector 65536) and write boot0 and U-Boot.
    DdA733,

    /// Flash `boot` and `root` partitions directly through `fastboot` over USB.
    Fastboot,

    /// Flash T-Head TH1520 (`uboot` staged to RAM, then `uboot`, `boot`, `root`) through `fastboot`.
    FastbootTh1520,

    /// Flash SpacemiT K1/K3 (factory bootloader binaries, `boot`, `root`) through `fastboot`.
    FastbootSpacemit,

    /// Write the Rockchip boot medium through `rkdeveloptool` in MaskROM mode.
    Rkdeveloptool,
}

impl FlashVia {
    pub const fn as_str(self) -> &'static str {
        match self {
            FlashVia::DdWhole => "dd-whole",
            FlashVia::IsoWrite => "iso-write",
            FlashVia::DdBootRoot => "dd-boot-root",
            FlashVia::DdEfiRoot => "dd-efi-root",
            FlashVia::DdRoot => "dd-root",
            FlashVia::DdTh1520 => "dd-th1520",
            FlashVia::DdA733 => "dd-a733",
            FlashVia::Fastboot => "fastboot",
            FlashVia::FastbootTh1520 => "fastboot-th1520",
            FlashVia::FastbootSpacemit => "fastboot-spacemit",
            FlashVia::Rkdeveloptool => "rkdeveloptool",
        }
    }

    /// Artifacts required by this write method, in flash order.
    pub const fn required_roles(self) -> &'static [Role] {
        match self {
            FlashVia::DdWhole => &[Role::Disk],
            FlashVia::IsoWrite => &[Role::Iso],
            FlashVia::DdBootRoot | FlashVia::DdTh1520 | FlashVia::Fastboot => {
                &[Role::Boot, Role::Root]
            }
            FlashVia::DdEfiRoot => &[Role::Efi, Role::Root],
            FlashVia::DdRoot => &[Role::Root],
            FlashVia::DdA733 => &[Role::Boot0, Role::Uboot, Role::Root],
            FlashVia::FastbootTh1520 => &[Role::Uboot, Role::Boot, Role::Root],
            FlashVia::FastbootSpacemit => &[
                Role::Bootinfo,
                Role::Fsbl,
                Role::Env,
                Role::Opensbi,
                Role::Uboot,
                Role::Boot,
                Role::Root,
            ],
            FlashVia::Rkdeveloptool => &[Role::Loader, Role::Root],
        }
    }
}

impl fmt::Display for FlashVia {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
