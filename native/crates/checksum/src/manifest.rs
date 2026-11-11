//! Parsing sha256sum manifests and finding them on disk.

use std::path::{Path, PathBuf};

use crate::{Error, Result};

/// Length of a hex SHA-256 digest.
const DIGEST_LEN: usize = 64;

/// Suffixes that mark a file as a manifest naming the file before them.
const SUFFIXES: [&str; 2] = [".sha256sum", ".sha256"];

/// The tool whose per-architecture mirror listings (`sha256sum_riscv64.txt`)
/// this crate reads.
const TOOL: &str = "sha256sum";

/// The stem a manifest file name is about, e.g. `image.tar.xz` for
/// `image.tar.xz.sha256sum`.
///
/// The mirror's per-architecture form (`sha256sum_riscv64.txt`) names no single
/// file rather than an empty one, so it yields `None`.
fn subject_of(file_name: &str) -> Option<&str> {
    let lower = file_name.to_ascii_lowercase();

    // The suffixes are ASCII, so removing their byte length cannot split a
    // multi-byte character of `file_name`.
    SUFFIXES
        .iter()
        .find_map(|suffix| {
            lower
                .ends_with(suffix)
                .then(|| &file_name[..file_name.len() - suffix.len()])
        })
        .filter(|stem| !stem.is_empty())
}

/// Whether `file_name` names a sha256 manifest.
fn is_manifest(file_name: &str) -> bool {
    subject_of(file_name).is_some() || tool_listing(file_name)
}

/// Whether `file_name` is the mirror's per-architecture listing, e.g.
/// `sha256sum_riscv64.txt` or `sha256sum.txt`.
fn tool_listing(file_name: &str) -> bool {
    let lower = file_name.to_ascii_lowercase();

    lower
        .strip_prefix(TOOL)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(['_', '.', '-']))
}

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
    fn parse(line: &str) -> Option<Self> {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }

        let (digest, file) = match line.split_once(char::is_whitespace) {
            Some((digest, rest)) => (digest, rest.trim_start_matches([' ', '\t', '*'])),
            None => (line, ""),
        };

        let digest = digest.to_ascii_lowercase();
        let valid =
            digest.len() == DIGEST_LEN && digest.bytes().all(|byte| byte.is_ascii_hexdigit());

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

        file_name(&self.file) == Some(name)
    }
}

/// The file name a manifest entry lists, ignoring any directory part.
fn file_name(listed: &str) -> Option<&str> {
    Path::new(listed).file_name().and_then(|name| name.to_str())
}

/// A parsed checksum manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// File the manifest was read from.
    path: PathBuf,

    /// Checksums it lists.
    entries: Vec<Entry>,
}

impl Manifest {
    /// Read the manifest at `path`, if its name marks it as one.
    pub fn read(path: &Path) -> Result<Self> {
        let is_manifest = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(is_manifest);

        if !is_manifest {
            return Err(Error::NotAManifest {
                path: path.to_path_buf(),
            });
        }

        let text = fsx::read_to_string(path)?;

        Ok(Self::parse(path.to_path_buf(), &text))
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
                .is_some_and(is_manifest);

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
        Self::load(fsx::files_in(directory)?)
    }

    pub(crate) fn parse(path: PathBuf, text: &str) -> Self {
        let entries = text.lines().filter_map(Entry::parse).collect();

        Self { path, entries }
    }

    /// The checksum this manifest records for `file`, if it records one.
    pub(crate) fn digest(&self, file: &Path) -> Option<&str> {
        let name = file.file_name().and_then(|name| name.to_str())?;
        let subject = self.subject();

        self.entries
            .iter()
            .find(|entry| entry.covers(name, subject))
            .map(|entry| entry.digest.as_str())
    }

    /// The file names this manifest covers, as they appear in the manifest.
    ///
    /// A lone digest covers the file the manifest is named after, e.g.
    /// `image.img` for `image.img.sha256`; other entries cover the file they
    /// list. Entries without a readable file name are skipped.
    pub fn files(&self) -> impl Iterator<Item = &str> + '_ {
        self.entries.iter().filter_map(move |entry| {
            if entry.file.is_empty() || entry.file == "-" {
                self.subject()
            } else {
                file_name(&entry.file)
            }
        })
    }

    /// The file a manifest is named after, e.g. `image.tar.xz` for
    /// `image.tar.xz.sha256sum`.
    fn subject(&self) -> Option<&str> {
        let name = self.path.file_name().and_then(|name| name.to_str())?;

        subject_of(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testutils::Workspace;

    /// A syntactically valid digest, for parser tests.
    fn fake() -> String {
        "ab".repeat(DIGEST_LEN / 2)
    }

    fn parse(text: &str) -> Manifest {
        Manifest::parse(PathBuf::from("image.sha256sum"), text)
    }

    #[test]
    fn detects_manifest_names() {
        for name in [
            "image.sha256sum",
            "image.tar.xz.sha256sum",
            "image.SHA256",
            "image.root.ext4.sha256",
            "sha256sum_riscv64.txt",
            "SHA256SUM_arm64.txt",
            "sha256sum",
        ] {
            assert!(is_manifest(name), "{name}");
        }

        for name in ["image.img", "image.tar.xz", "README.md", "sha256sums.txt"] {
            assert!(!is_manifest(name), "{name}");
        }
    }

    #[test]
    fn strips_the_matching_suffix() {
        assert_eq!(subject_of("image.tar.xz.sha256sum"), Some("image.tar.xz"));
        assert_eq!(
            subject_of("image.root.ext4.SHA256"),
            Some("image.root.ext4")
        );
        assert_eq!(subject_of("sha256sum_riscv64.txt"), None);
        assert_eq!(subject_of(".sha256"), None);
    }

    #[test]
    fn parses_coreutils_lines() {
        let sha = fake();
        let text = format!(
            "# generated by sha256sum\n\
             \n\
             {sha}  image.root.ext4\n\
             {sha} *image.boot.ext4\n\
             {sha}\n\
             nonsense\n"
        );

        let manifest = parse(&text);
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
        let manifest = parse("abcd  image.img\n");
        assert!(manifest.entries.is_empty());
    }

    #[test]
    fn matches_entries_by_file_name() {
        let sha = fake();
        let manifest = parse(&format!(
            "{sha}  ./deepin-k1-riscv64-25-desktop-installer.root.ext4\n"
        ));

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
        let sha = fake();
        let manifest = Manifest::parse(
            PathBuf::from("deepin-25-crimson-preview-arm64-phytiumpi.tar.xz.sha256"),
            &sha,
        );

        assert_eq!(
            manifest.digest(Path::new(
                "deepin-25-crimson-preview-arm64-phytiumpi.tar.xz"
            )),
            Some(sha.as_str())
        );
        assert_eq!(manifest.digest(Path::new("something-else.img")), None);
    }

    #[test]
    fn lists_the_files_it_covers() {
        let sha = fake();
        let manifest = parse(&format!(
            "{sha}  image.root.ext4\n\
             {sha}  ./image.boot.ext4\n\
             {sha}  -\n\
             {sha}\n"
        ));

        let files: Vec<&str> = manifest.files().collect();
        assert_eq!(
            files,
            ["image.root.ext4", "image.boot.ext4", "image", "image"]
        );

        let lone = Manifest::parse(PathBuf::from("image.root.ext4.sha256"), &sha);
        let files: Vec<&str> = lone.files().collect();
        assert_eq!(files, ["image.root.ext4"]);
    }

    #[test]
    fn loads_only_manifests_from_paths() {
        let root = Workspace::new("deepin-ports-manifest", "loads");

        let sum = fake();
        std::fs::write(root.join("image.img"), b"payload").unwrap();
        std::fs::write(root.join("image.sha256sum"), format!("{sum}  image.img\n")).unwrap();

        let paths = vec![root.join("image.img"), root.join("image.sha256sum")];
        let manifests = Manifest::load(paths).unwrap();

        assert_eq!(manifests.len(), 1);
        assert_eq!(
            manifests[0].digest(Path::new("image.img")),
            Some(sum.as_str())
        );
    }

    #[test]
    fn loads_manifests_from_a_directory() {
        let root = Workspace::new("deepin-ports-manifest", "in-directory");

        let sum = fake();
        std::fs::write(root.join("image.tar.xz"), b"payload").unwrap();
        std::fs::write(
            root.join("image.tar.xz.sha256sum"),
            format!("{sum}  image.tar.xz\n"),
        )
        .unwrap();
        std::fs::write(
            root.join("sha256sum_riscv64.txt"),
            format!("{sum}  image.tar.xz\n"),
        )
        .unwrap();
        std::fs::write(root.join("README.md"), b"not a manifest").unwrap();
        std::fs::create_dir_all(root.join("subdir")).unwrap();

        let manifests = Manifest::in_directory(&root).unwrap();
        assert_eq!(manifests.len(), 2);
    }

    #[test]
    fn rejects_names_that_are_not_manifests() {
        assert!(matches!(
            Manifest::read(Path::new("/tmp/image.img")),
            Err(Error::NotAManifest { .. })
        ));
    }
}
