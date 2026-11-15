use std::io::Read;

use crate::fsx::Amount;

pub const BUFFER_SIZE: usize = 1024 * 1024;

/// `Read` adapter that reports cumulative bytes consumed via `progress`.
#[derive(Debug)]
pub struct Chunked<R, F> {
    inner: R,
    done: u64,
    total: Option<u64>,
    progress: F,
}

impl<R, F> Chunked<R, F> {
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
