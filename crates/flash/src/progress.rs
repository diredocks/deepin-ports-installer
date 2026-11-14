//! Progress reported while preparing an image, and while the helper writes it.
//!
//! Byte counts are the shared [`Amount`], so extraction, verification and the
//! write reports the helper streams back are rendered the same way.

use fsx::Amount;

/// Stage of [`prepare`](crate::prepare) a [`Progress`] report belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Unpacking the archive.
    Extract,

    /// Hashing the unpacked artifacts.
    Verify,

    /// Writing the artifacts to the target.
    ///
    /// This phase belongs to the privileged helper: the reports arrive as
    /// [`Amount`] values over the sidecar protocol and are wrapped here so a
    /// frontend renders them like its own phases.
    Write,
}

impl Phase {
    /// A short label, suitable for a progress bar.
    pub const fn as_str(self) -> &'static str {
        match self {
            Phase::Extract => "extracting",
            Phase::Verify => "verifying",
            Phase::Write => "writing",
        }
    }
}

/// Work [`prepare`](crate::prepare) has done in its current phase.
///
/// The byte counts are the shared [`Amount`], so a frontend renders extraction,
/// verification and downloading progress the same way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    /// Phase being reported.
    pub phase: Phase,

    /// Bytes processed so far in this phase, and the total expected in it.
    pub amount: Amount,
}
