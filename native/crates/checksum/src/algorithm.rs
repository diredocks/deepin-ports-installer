use std::fmt;

/// Manifest file-name suffixes and the algorithm each names.
const SUFFIXES: [(&str, Algorithm); 6] = [
    (".sha256sum", Algorithm::Sha256),
    (".b2sum", Algorithm::Blake2b),
    (".md5sum", Algorithm::Md5),
    (".sha256", Algorithm::Sha256),
    (".blake2b", Algorithm::Blake2b),
    (".md5", Algorithm::Md5),
];

/// Hash function used by a checksum manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Algorithm {
    /// MD5 (`md5sum`), the weakest algorithm still published today.
    Md5,

    /// BLAKE2b-512 (`b2sum`), published alongside some archives.
    Blake2b,

    /// SHA-256 (`sha256sum`), the default.
    Sha256,
}

impl Algorithm {
    /// The coreutils name of this algorithm.
    pub const fn as_str(self) -> &'static str {
        match self {
            Algorithm::Md5 => "md5",
            Algorithm::Blake2b => "blake2b",
            Algorithm::Sha256 => "sha256",
        }
    }

    /// The length of the hex digest this algorithm produces.
    pub(crate) const fn hex_len(self) -> usize {
        match self {
            Algorithm::Md5 => 32,
            Algorithm::Blake2b => 128,
            Algorithm::Sha256 => 64,
        }
    }

    /// Rank used to prefer the strongest of several manifests.
    pub(crate) const fn strength(self) -> u8 {
        match self {
            Algorithm::Md5 => 0,
            Algorithm::Blake2b => 1,
            Algorithm::Sha256 => 2,
        }
    }

    /// The algorithm a manifest file name advertises, if any.
    ///
    /// Both the coreutils (`*.sha256sum`) and the shorter (`*.sha256`)
    /// spellings are recognized, case-insensitively, as are the
    /// per-architecture manifests the mirror publishes
    /// (`sha256sum_riscv64.txt`).
    pub fn from_file_name(file_name: &str) -> Option<Self> {
        if let Some((_, algorithm)) = Self::detect(file_name) {
            return Some(algorithm);
        }

        let lower = file_name.to_ascii_lowercase();
        SUFFIXES
            .iter()
            .filter(|(suffix, _)| suffix.ends_with("sum"))
            .find(|(suffix, _)| prefixed(&lower, &suffix[1..]))
            .map(|(_, algorithm)| *algorithm)
    }

    /// The suffix that made `file_name` a manifest, with its algorithm.
    pub(crate) fn detect(file_name: &str) -> Option<(&str, Self)> {
        let lower = file_name.to_ascii_lowercase();
        let (suffix, algorithm) = SUFFIXES
            .iter()
            .find(|(suffix, _)| lower.ends_with(suffix))?;

        Some((&file_name[..file_name.len() - suffix.len()], *algorithm))
    }
}

/// Whether `name` starts with `tool` followed by a word boundary.
fn prefixed(name: &str, tool: &str) -> bool {
    name.strip_prefix(tool)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(['_', '.', '-']))
}

impl fmt::Display for Algorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A checksum recorded for a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Digest {
    /// Algorithm that produced the checksum.
    pub algorithm: Algorithm,

    /// Lowercase hex checksum.
    pub value: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_manifest_algorithms() {
        for (name, algorithm) in [
            ("image.sha256sum", Algorithm::Sha256),
            ("image.tar.xz.sha256sum", Algorithm::Sha256),
            ("image.SHA256", Algorithm::Sha256),
            ("image.b2sum", Algorithm::Blake2b),
            ("image.md5sum", Algorithm::Md5),
            ("image.md5", Algorithm::Md5),
            ("sha256sum_riscv64.txt", Algorithm::Sha256),
            ("MD5SUM_arm64.txt", Algorithm::Md5),
            ("b2sum", Algorithm::Blake2b),
        ] {
            assert_eq!(Algorithm::from_file_name(name), Some(algorithm), "{name}");
        }

        for name in ["image.img", "image.tar.xz", "README.md", "sha256sums.txt"] {
            assert_eq!(Algorithm::from_file_name(name), None, "{name}");
        }
    }

    #[test]
    fn strips_the_matching_suffix() {
        assert_eq!(
            Algorithm::detect("image.tar.xz.b2sum"),
            Some(("image.tar.xz", Algorithm::Blake2b))
        );
        assert_eq!(
            Algorithm::detect("image.root.ext4.sha256"),
            Some(("image.root.ext4", Algorithm::Sha256))
        );
    }

    #[test]
    fn ranks_algorithms_by_strength() {
        assert!(Algorithm::Sha256.strength() > Algorithm::Blake2b.strength());
        assert!(Algorithm::Blake2b.strength() > Algorithm::Md5.strength());
    }
}
