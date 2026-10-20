//! Stream a remote file to a local path, resuming an interrupted transfer when
//! the server supports HTTP `Range` requests.

use std::path::{Path, PathBuf};

use url::Url;

use crate::paths;
use crate::{Error, Result};

/// Resolve where a download from `url` should be written.
///
/// A destination that is an existing directory, or that ends with a path
/// separator, has the file name from `url` appended. Anything else is used
/// verbatim. Without a destination, the file is placed in the `images`
/// subdirectory of the platform cache directory.
pub fn destination(output: Option<&Path>, url: &str) -> Result<PathBuf> {
    let file_name = file_name(url).ok_or_else(|| Error::LinkFileName(url.to_owned()))?;

    match output {
        None => Ok(paths::cache_dir()?.join("images").join(file_name)),
        Some(path) if paths::is_directory(path) => Ok(path.join(file_name)),
        Some(path) => Ok(path.to_path_buf()),
    }
}

/// Progress reported while downloading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    /// Bytes written so far.
    pub downloaded: u64,

    /// Total size in bytes, when known.
    pub total: Option<u64>,
}

/// Download `url` to `dest`, calling `progress` as data arrives.
///
/// `size` is the expected length in bytes, or `0` when it is unknown. The body
/// is streamed to a temporary sibling of `dest` and moved into place once
/// complete, so `dest` never holds a partial download. An interrupted download
/// leaves that sibling behind; a later call resumes it with an HTTP `Range`
/// request instead of starting over. Unless `force` is set, an existing `dest`
/// is refused.
#[cfg(feature = "http")]
pub async fn download<F>(
    url: &str,
    size: u64,
    dest: &Path,
    force: bool,
    mut progress: F,
) -> Result<()>
where
    F: FnMut(Progress),
{
    if !force && dest.exists() {
        return Err(Error::AlreadyExists {
            path: dest.to_path_buf(),
        });
    }

    if let Some(parent) = dest
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(|source| Error::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let temp = temp_path(dest);
    let expected = (size > 0).then_some(size);

    // Resume wherever a previous run stopped, unless the leftover is already
    // as large as the file we expect (a stale partial from another build).
    let mut offset = partial_size(&temp);
    if expected.is_some_and(|total| offset >= total) {
        let _ = std::fs::remove_file(&temp);
        offset = 0;
    }

    let response = crate::http::response_from(url, offset).await?;
    let (response, offset) = reconcile(response, url, &temp, offset).await?;

    let total = response
        .content_length()
        .map(|length| length + offset)
        .or(expected);

    let mut file = open_partial(&temp, offset)?;
    progress(Progress {
        downloaded: offset,
        total,
    });

    let written = match write_body(
        &mut file,
        response,
        &temp,
        url,
        offset,
        total,
        &mut progress,
    )
    .await
    {
        Ok(written) => written,
        Err(error) => {
            let _ = std::io::Write::flush(&mut file);
            return Err(error);
        }
    };

    if let Err(source) = std::io::Write::flush(&mut file) {
        drop(file);
        return Err(Error::Write { path: temp, source });
    }
    drop(file);

    if let Some(expected) = expected
        && written < expected
    {
        return Err(Error::Incomplete {
            url: url.to_owned(),
            downloaded: written,
            expected,
        });
    }

    if force && dest.exists() {
        std::fs::remove_file(dest).map_err(|source| Error::Write {
            path: dest.to_path_buf(),
            source,
        })?;
    }

    std::fs::rename(&temp, dest).map_err(|source| Error::Write {
        path: dest.to_path_buf(),
        source,
    })?;

    Ok(())
}

/// Reconcile a possibly-ranged response with the offset we asked to resume
/// from, returning the response to stream and the offset its body starts at.
///
/// A server that ignores the range, or reports it unsatisfiable, is handled by
/// restarting from the beginning so the resulting file is always contiguous.
#[cfg(feature = "http")]
async fn reconcile(
    response: reqwest::Response,
    url: &str,
    temp: &Path,
    offset: u64,
) -> Result<(reqwest::Response, u64)> {
    use reqwest::StatusCode;

    if offset == 0 {
        return ensure_success(response, url).map(|response| (response, 0));
    }

    match response.status() {
        // A genuine resume: the body starts exactly where the part file ends.
        StatusCode::PARTIAL_CONTENT if content_range_start(&response) == Some(offset) => {
            Ok((response, offset))
        }
        // A partial response we cannot place, or a range the server refuses:
        // drop the part file and fetch the whole file again.
        StatusCode::PARTIAL_CONTENT | StatusCode::RANGE_NOT_SATISFIABLE => {
            let _ = std::fs::remove_file(temp);
            let response = crate::http::response_from(url, 0).await?;
            ensure_success(response, url).map(|response| (response, 0))
        }
        // The server ignored the range and sent the whole file; reuse its body.
        status if status.is_success() => Ok((response, 0)),
        status => Err(Error::HttpStatus {
            url: url.to_owned(),
            status,
        }),
    }
}

/// Return `response` unchanged when it has a success status, or the matching
/// [`Error::HttpStatus`] otherwise.
#[cfg(feature = "http")]
fn ensure_success(response: reqwest::Response, url: &str) -> Result<reqwest::Response> {
    let status = response.status();
    if status.is_success() {
        Ok(response)
    } else {
        Err(Error::HttpStatus {
            url: url.to_owned(),
            status,
        })
    }
}

/// The first byte position advertised by a `Content-Range` header.
///
/// `bytes 100-499/1234` yields `100`; a missing or malformed header yields
/// `None`.
#[cfg(feature = "http")]
fn content_range_start(response: &reqwest::Response) -> Option<u64> {
    response
        .headers()
        .get(reqwest::header::CONTENT_RANGE)?
        .to_str()
        .ok()
        .and_then(parse_content_range)
}

/// Parse the first byte position out of a `Content-Range` value.
///
/// The `bytes` unit is matched case-insensitively, as HTTP tokens are.
#[cfg(any(feature = "http", test))]
fn parse_content_range(value: &str) -> Option<u64> {
    let (unit, range) = value.split_once(' ')?;
    if !unit.eq_ignore_ascii_case("bytes") {
        return None;
    }

    range.split(['-', '/']).next()?.trim().parse().ok()
}

/// Open the temporary file for writing, appending when resuming from `offset`.
#[cfg(feature = "http")]
fn open_partial(path: &Path, offset: u64) -> Result<std::fs::File> {
    let file = if offset == 0 {
        std::fs::File::create(path)
    } else {
        std::fs::OpenOptions::new().append(true).open(path)
    };

    file.map_err(|source| Error::Write {
        path: path.to_path_buf(),
        source,
    })
}

/// Size of an existing partial download, or zero when there is none.
#[cfg(feature = "http")]
fn partial_size(path: &Path) -> u64 {
    std::fs::metadata(path)
        .map(|metadata| metadata.len())
        .unwrap_or(0)
}

/// Stream a response body into `file`, reporting progress.
///
/// `downloaded` seeds the running count with the bytes already on disk so
/// progress and the returned total include a resumed prefix.
#[cfg(feature = "http")]
async fn write_body<F>(
    file: &mut std::fs::File,
    mut response: reqwest::Response,
    dest: &Path,
    url: &str,
    mut downloaded: u64,
    total: Option<u64>,
    progress: &mut F,
) -> Result<u64>
where
    F: FnMut(Progress),
{
    use std::io::Write;

    while let Some(chunk) = response.chunk().await.map_err(|source| Error::Request {
        url: url.to_owned(),
        source,
    })? {
        file.write_all(&chunk).map_err(|source| Error::Write {
            path: dest.to_path_buf(),
            source,
        })?;

        downloaded += chunk.len() as u64;
        progress(Progress { downloaded, total });
    }

    Ok(downloaded)
}

/// The final path segment of a link, ignoring any query or fragment.
fn file_name(link: &str) -> Option<String> {
    Url::parse(link)
        .ok()?
        .path_segments()?
        .next_back()
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
}

/// A stable temporary sibling of `dest`, used both for the in-flight download
/// and to resume an interrupted one on a later run.
#[cfg(any(feature = "http", test))]
fn temp_path(dest: &Path) -> PathBuf {
    let mut name = std::ffi::OsString::from(".");
    name.push(dest.file_name().map_or_else(
        || std::ffi::OsString::from("download"),
        |name| name.to_os_string(),
    ));
    name.push(".part");
    dest.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_file_name_from_link() {
        assert_eq!(
            file_name("https://e/a/b.tar.xz").as_deref(),
            Some("b.tar.xz")
        );
        assert_eq!(
            file_name("https://e/a/b.tar.xz?token=1#frag").as_deref(),
            Some("b.tar.xz")
        );
        assert_eq!(file_name("https://e/").as_deref(), None);
        assert_eq!(file_name("https://e").as_deref(), None);
        assert_eq!(file_name("not a url").as_deref(), None);
    }

    #[test]
    fn builds_destination_from_link_and_output() {
        let link = "https://e/20250101/riscv64/deepin-25-crimson-preview-riscv64-k1-20250101-000000.tar.xz";

        assert_eq!(
            destination(None, link).unwrap(),
            paths::cache_dir()
                .unwrap()
                .join("images")
                .join("deepin-25-crimson-preview-riscv64-k1-20250101-000000.tar.xz")
        );

        assert_eq!(
            destination(Some(Path::new("/tmp/custom.img")), link).unwrap(),
            PathBuf::from("/tmp/custom.img")
        );

        assert_eq!(
            destination(Some(&std::env::temp_dir()), link).unwrap(),
            std::env::temp_dir()
                .join("deepin-25-crimson-preview-riscv64-k1-20250101-000000.tar.xz")
        );

        assert!(matches!(
            destination(None, "https://e/"),
            Err(Error::LinkFileName(_))
        ));
    }

    #[test]
    fn temporary_path_is_a_sibling() {
        let temp = temp_path(Path::new("/tmp/image.tar.xz"));
        assert_eq!(temp, Path::new("/tmp/.image.tar.xz.part"));
    }

    #[test]
    fn parses_content_range_start() {
        assert_eq!(parse_content_range("bytes 100-499/1234"), Some(100));
        assert_eq!(parse_content_range("Bytes 12-34/*"), Some(12));
        assert_eq!(parse_content_range("bytes 0-0/1"), Some(0));
        assert_eq!(parse_content_range("bytes 12-34/*"), Some(12));
        assert_eq!(parse_content_range("100-499/1234"), None);
        assert_eq!(parse_content_range("bytes */1234"), None);
        assert_eq!(parse_content_range("nonsense"), None);
    }
}
