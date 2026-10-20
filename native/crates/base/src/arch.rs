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
    /// Every architecture, in the order used by the generated `images.toml`.
    pub const ALL: [Arch; 4] = [Arch::Riscv, Arch::Arm64, Arch::Amd64, Arch::Loong64];

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

impl TryFrom<&str> for Arch {
    type Error = Error;

    fn try_from(slug: &str) -> Result<Self> {
        slug.parse()
    }
}

impl From<Arch> for &'static str {
    fn from(arch: Arch) -> Self {
        arch.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_round_trips() {
        for arch in Arch::ALL {
            assert_eq!(Arch::from_str(arch.as_str()).unwrap(), arch);
            assert_eq!(arch.to_string(), arch.as_str());
            assert_eq!(<&str>::from(arch), arch.as_str());
        }
    }

    #[test]
    fn rejects_unknown_slug() {
        assert!(matches!(
            Arch::from_str("mips"),
            Err(Error::UnknownArch(slug)) if slug == "mips"
        ));
    }

    #[test]
    fn deserializes_from_toml_slug() {
        #[derive(Deserialize)]
        struct Wrapper {
            arch: Arch,
        }

        let wrapper: Wrapper = toml::from_str("arch = \"arm64\"").unwrap();
        assert_eq!(wrapper.arch, Arch::Arm64);
    }
}
