use std::fmt;

use serde::{Deserialize, Serialize};

/// How an image is written to a medium.
///
/// The names are the `flash_via` values of `devices.toml`, one per write driver
/// of the privileged helper: the catalogue only records *which* method a device
/// uses, the driver that implements it lives there.
///
/// A device may list several methods — eic7700 writes to SD cards and SATA/NVMe
/// drives through partitions but to eMMC through fastboot — in preference
/// order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
#[serde(rename_all = "kebab-case")]
pub enum FlashVia {
    /// Write a single whole-disk image, e.g. jh7110 and phytiumpi.
    DdWhole,

    /// Partition the medium, then write each partition image in place.
    DdPartitions,

    /// Flash named partitions through `fastboot` over USB.
    Fastboot,

    /// Write images to the boot medium through `rkdeveloptool` in MaskROM mode.
    Rkdeveloptool,

    /// Write a bootable ISO to removable media, e.g. the generic builds.
    IsoWrite,
}

impl FlashVia {
    /// The `devices.toml` slug of this method, e.g. `dd-whole`.
    pub const fn as_str(self) -> &'static str {
        match self {
            FlashVia::DdWhole => "dd-whole",
            FlashVia::DdPartitions => "dd-partitions",
            FlashVia::Fastboot => "fastboot",
            FlashVia::Rkdeveloptool => "rkdeveloptool",
            FlashVia::IsoWrite => "iso-write",
        }
    }
}

impl fmt::Display for FlashVia {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
