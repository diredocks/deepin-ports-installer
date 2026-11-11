//! Bytes processed by a long-running operation.

/// Work done so far, and how much is expected.
///
/// One type serves every operation that streams data: downloading a file,
/// unpacking an archive, hashing an artifact. Operations that run in phases
/// carry an [`Amount`] next to the phase instead of duplicating the pair.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Amount {
    /// Bytes processed so far.
    pub done: u64,

    /// Total bytes expected, when known.
    pub total: Option<u64>,
}

impl Amount {
    /// An amount with `done` bytes processed out of `total`.
    pub const fn new(done: u64, total: Option<u64>) -> Self {
        Self { done, total }
    }

    /// Progress in whole percent, when the total is known and non-zero.
    pub fn percent(self) -> Option<u64> {
        self.total
            .filter(|total| *total > 0)
            .map(|total| self.done.saturating_mul(100) / total)
    }
}
