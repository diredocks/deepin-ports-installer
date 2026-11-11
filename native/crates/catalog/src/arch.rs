use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// A CPU architecture supported by Deepin Ports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
#[serde(rename_all = "lowercase")]
pub enum Arch {
    Riscv,
    Arm64,
    Amd64,
    Loong64,
}

impl Arch {
    /// The TOML slug of this architecture, e.g. `riscv`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Arch::Riscv => "riscv",
            Arch::Arm64 => "arm64",
            Arch::Amd64 => "amd64",
            Arch::Loong64 => "loong64",
        }
    }
}

impl fmt::Display for Arch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Arch {
    type Err = Error;

    fn from_str(slug: &str) -> Result<Self> {
        match slug {
            "riscv" => Ok(Arch::Riscv),
            "arm64" => Ok(Arch::Arm64),
            "amd64" => Ok(Arch::Amd64),
            "loong64" => Ok(Arch::Loong64),
            other => Err(Error::UnknownArch(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_round_trips() {
        for arch in [Arch::Riscv, Arch::Arm64, Arch::Amd64, Arch::Loong64] {
            assert_eq!(Arch::from_str(arch.as_str()).unwrap(), arch);
        }
    }

    #[test]
    fn rejects_unknown_slug() {
        assert!(matches!(
            Arch::from_str("mips"),
            Err(Error::UnknownArch(slug)) if slug == "mips"
        ));
    }
}
