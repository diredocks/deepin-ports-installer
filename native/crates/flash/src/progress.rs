//! Progress reported while preparing an image.

/// Stage of [`prepare`](crate::prepare) a [`Progress`] report belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Unpacking the archive.
    Extract,

    /// Hashing the unpacked artifacts.
    Verify,
}

impl Phase {
    /// A short label, suitable for a progress bar.
    pub const fn as_str(self) -> &'static str {
        match self {
            Phase::Extract => "extracting",
            Phase::Verify => "verifying",
        }
    }
}

/// Work [`prepare`](crate::prepare) has done in its current phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    /// Phase being reported.
    pub phase: Phase,

    /// Bytes processed so far in this phase.
    pub done: u64,

    /// Total bytes expected in this phase, when known.
    pub total: Option<u64>,
}
