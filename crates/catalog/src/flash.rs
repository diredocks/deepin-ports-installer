use std::fmt;

use serde::{Deserialize, Serialize};

/// How an image is written to a medium, as named by `flash_via` in `devices.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
#[serde(rename_all = "kebab-case")]
pub enum FlashVia {
    /// Write a single whole-disk image.
    DdWhole,

    /// Partition the medium, then write each partition image in place.
    DdPartitions,

    /// Flash named partitions through `fastboot` over USB.
    Fastboot,

    /// Write the boot medium through `rkdeveloptool` in MaskROM mode.
    Rkdeveloptool,

    /// Write a bootable ISO to removable media.
    IsoWrite,
}

impl FlashVia {
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
