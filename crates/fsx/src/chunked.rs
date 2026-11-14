//! Reading a stream in chunks while reporting progress.

use std::io::Read;

use crate::Amount;

/// Read buffer size used for streaming operations.
pub const BUFFER_SIZE: usize = 1024 * 1024;

/// A reader that reports how much of the stream has been consumed.
///
/// `total` is the expected length in bytes, when known; it only fills
/// [`Amount::total`]. Every non-empty read reports the running total, so a
/// caller can hash, copy or unpack the bytes it read and still report progress.
#[derive(Debug)]
pub struct Chunked<R, F> {
    inner: R,
    done: u64,
    total: Option<u64>,
    progress: F,
}

impl<R, F> Chunked<R, F> {
    /// Report `progress` as `inner` is read, against an expected `total`.
    pub fn new(inner: R, total: Option<u64>, progress: F) -> Self {
        Self {
            inner,
            done: 0,
            total,
            progress,
        }
    }
}

impl<R: Read, F: FnMut(Amount)> Read for Chunked<R, F> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let read = self.inner.read(buffer)?;

        if read > 0 {
            self.done += read as u64;
            (self.progress)(Amount::new(self.done, self.total));
        }

        Ok(read)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_every_non_empty_read() {
        let reader = std::io::Cursor::new(vec![0u8; 3]);
        let mut reports = Vec::new();

        let mut chunked = Chunked::new(reader, Some(3), |amount| reports.push(amount));
        let mut buffer = [0u8; 2];

        assert_eq!(chunked.read(&mut buffer).unwrap(), 2);
        assert_eq!(chunked.read(&mut buffer).unwrap(), 1);
        assert_eq!(chunked.read(&mut buffer).unwrap(), 0);
        drop(chunked);

        assert_eq!(reports, [Amount::new(2, Some(3)), Amount::new(3, Some(3))]);
    }
}
