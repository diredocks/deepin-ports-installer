//! Parsing checksum manifests and finding them on disk.

use std::path::{Path, PathBuf};

use crate::{Algorithm, Error, Result};

/// One `<checksum>  <file>` line of a manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    /// Lowercase hex checksum.
    digest: String,

    /// File the checksum covers; empty for a lone digest.
    file: String,
}

impl Entry {
    /// Parse one manifest line; `None` for blanks, comments and non-checksums.
    fn parse(line: &str, algorithm: Algorithm) -> Option<Self> {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }

        let (digest, file) = match line.split_once(char::is_whitespace) {
            Some((digest, rest)) => (digest, rest.trim_start_matches([' ', '\t', '*'])),
            None => (line, ""),
        };

        let digest = digest.to_ascii_lowercase();
        let valid = digest.len() == algorithm.hex_len()
            && digest.bytes().all(|byte| byte.is_ascii_hexdigit());

        valid.then(|| Entry {
            digest,
            file: file.to_owned(),
        })
    }

    /// Whether this entry covers `name`, the file name of an artifact.
    ///
    /// A lone digest (`<checksum>`, `<checksum>  -`) covers the file the
    /// manifest is named after instead of an explicitly listed file.
    fn covers(&self, name: &str, subject: Option<&str>) -> bool {
        if self.file.is_empty() || self.file == "-" {
            return subject == Some(name);
        }

        Path::new(&self.file)
            .file_name()
            .and_then(|listed| listed.to_str())
            == Some(name)
    }
}

/// A parsed checksum manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// File the manifest was read from.
    path: PathBuf,

    /// Hash function its checksums use.
    algorithm: Algorithm,

    /// Checksums it lists.
    entries: Vec<Entry>,
}

impl Manifest {
    /// Read the manifest at `path`, detecting its algorithm from the file name.
    pub fn read(path: &Path) -> Result<Self> {
        let algorithm = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(Algorithm::from_file_name)
            .ok_or_else(|| Error::UnknownManifest {
                path: path.to_path_buf(),
            })?;

        let text = std::fs::read_to_string(path).map_err(|source| Error::Read {
            path: path.to_path_buf(),
            source,
        })?;

        Ok(Self::parse(path.to_path_buf(), algorithm, &text))
    }

    /// Load the manifests among `paths`, ignoring files that are not manifests.
    ///
    /// Callers can pass every file a directory listing produced.
    pub fn load<I>(paths: I) -> Result<Vec<Self>>
    where
        I: IntoIterator<Item = PathBuf>,
    {
        let mut manifests = Vec::new();

        for path in paths {
            let is_manifest = path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| Algorithm::from_file_name(name).is_some());

            if is_manifest {
                manifests.push(Self::read(&path)?);
            }
        }

        Ok(manifests)
    }

    /// Load the manifests in `directory`, ignoring other files.
    ///
    /// The directory is not searched recursively; callers that need manifests
    /// below the top level collect the paths themselves and use [`load`](Self::load).
    pub fn in_directory(directory: &Path) -> Result<Vec<Self>> {
        let mut paths = Vec::new();

        for entry in std::fs::read_dir(directory).map_err(|source| Error::Read {
            path: directory.to_path_buf(),
            source,
        })? {
            let entry = entry.map_err(|source| Error::Read {
                path: directory.to_path_buf(),
                source,
            })?;

            let file_type = entry.file_type().map_err(|source| Error::Read {
                path: entry.path(),
                source,
            })?;

            if file_type.is_file() {
                paths.push(entry.path());
            }
        }

        paths.sort();
        Self::load(paths)
    }

    pub(crate) fn parse(path: PathBuf, algorithm: Algorithm, text: &str) -> Self {
        let entries = text
            .lines()
            .filter_map(|line| Entry::parse(line, algorithm))
            .collect();

        Self {
            path,
            algorithm,
            entries,
        }
    }

    /// The hash function this manifest's checksums use.
    pub fn algorithm(&self) -> Algorithm {
        self.algorithm
    }

    /// The checksum this manifest records for `file`, if it records one.
    pub(crate) fn digest(&self, file: &Path) -> Option<&str> {
        let name = file.file_name().and_then(|name| name.to_str())?;
        let subject = self.subject();

        self.entries
            .iter()
            .find(|entry| entry.covers(name, subject.as_deref()))
            .map(|entry| entry.digest.as_str())
    }

    /// The file a manifest is named after, e.g. `image.tar.xz` for
    /// `image.tar.xz.b2sum`.
    fn subject(&self) -> Option<String> {
        let name = self.path.file_name().and_then(|name| name.to_str())?;
        let (stem, _) = Algorithm::detect(name)?;
        (!stem.is_empty()).then(|| stem.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A syntactically valid digest of `algorithm`, for parser tests.
    fn fake(algorithm: Algorithm) -> String {
        "ab".repeat(algorithm.hex_len() / 2)
    }

    fn parse(algorithm: Algorithm, text: &str) -> Manifest {
        Manifest::parse(
            PathBuf::from(format!("image.{}", algorithm.as_str())),
            algorithm,
            text,
        )
    }

    #[test]
    fn parses_coreutils_lines() {
        let sha = fake(Algorithm::Sha256);
        let text = format!(
            "# generated by sha256sum\n\
             \n\
             {sha}  image.root.ext4\n\
             {sha} *image.boot.ext4\n\
             {sha}\n\
             nonsense\n"
        );

        let manifest = parse(Algorithm::Sha256, &text);
        let files: Vec<&str> = manifest
            .entries
            .iter()
            .map(|entry| entry.file.as_str())
            .collect();

        assert_eq!(files, ["image.root.ext4", "image.boot.ext4", ""]);
        assert!(manifest.entries.iter().all(|entry| entry.digest == sha));
    }

    #[test]
    fn drops_lines_with_the_wrong_digest_length() {
        let manifest = parse(Algorithm::Sha256, "abcd  image.img\n");
        assert!(manifest.entries.is_empty());
    }

    #[test]
    fn matches_entries_by_file_name() {
        let sha = fake(Algorithm::Sha256);
        let manifest = parse(
            Algorithm::Sha256,
            &format!("{sha}  ./deepin-k1-riscv64-25-desktop-installer.root.ext4\n"),
        );

        assert_eq!(
            manifest.digest(Path::new(
                "/cache/unpacked/deepin-k1-riscv64-25-desktop-installer.root.ext4"
            )),
            Some(sha.as_str())
        );
        assert_eq!(manifest.digest(Path::new("/cache/other.img")), None);
    }

    #[test]
    fn lone_digests_cover_the_manifest_subject() {
        let sum = fake(Algorithm::Blake2b);
        let manifest = Manifest::parse(
            PathBuf::from("deepin-25-crimson-preview-arm64-phytiumpi.tar.xz.b2sum"),
            Algorithm::Blake2b,
            &sum,
        );

        assert_eq!(
            manifest.digest(Path::new(
                "deepin-25-crimson-preview-arm64-phytiumpi.tar.xz"
            )),
            Some(sum.as_str())
        );
        assert_eq!(manifest.digest(Path::new("something-else.img")), None);
    }

    #[test]
    fn loads_only_manifests_from_paths() {
        let root =
            std::env::temp_dir().join(format!("deepin-ports-manifest-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let sum = fake(Algorithm::Sha256);
        std::fs::write(root.join("image.img"), b"payload").unwrap();
        std::fs::write(root.join("image.sha256sum"), format!("{sum}  image.img\n")).unwrap();

        let paths = vec![root.join("image.img"), root.join("image.sha256sum")];
        let manifests = Manifest::load(paths).unwrap();

        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].algorithm(), Algorithm::Sha256);

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn loads_manifests_from_a_directory() {
        let root =
            std::env::temp_dir().join(format!("deepin-ports-in-directory-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let sum = fake(Algorithm::Blake2b);
        std::fs::write(root.join("image.tar.xz"), b"payload").unwrap();
        std::fs::write(
            root.join("image.tar.xz.b2sum"),
            format!("{sum}  image.tar.xz\n"),
        )
        .unwrap();
        std::fs::write(
            root.join("sha256sum_riscv64.txt"),
            format!("{}  image.tar.xz\n", fake(Algorithm::Sha256)),
        )
        .unwrap();
        std::fs::write(root.join("README.md"), b"not a manifest").unwrap();
        std::fs::create_dir_all(root.join("subdir")).unwrap();

        let manifests = Manifest::in_directory(&root).unwrap();
        let algorithms: Vec<Algorithm> = manifests
            .iter()
            .map(|manifest| manifest.algorithm())
            .collect();

        assert_eq!(algorithms, [Algorithm::Blake2b, Algorithm::Sha256]);

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn rejects_names_without_an_algorithm() {
        assert!(matches!(
            Manifest::read(Path::new("/tmp/image.img")),
            Err(Error::UnknownManifest { .. })
        ));
    }
}
