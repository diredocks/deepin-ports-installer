//! Orchestration shared by every Deepin Ports installer frontend.
//!
//! The [`cli`] binary and a future graphical installer drive the same
//! operations, so the rules for "which image, which device, in which order"
//! live here rather than in either frontend: [`resolve`] decides which image to
//! use, [`prepare`] turns the downloaded file into artifacts ready to be written,
//! and [`plan`] describes the privileged write that follows.
//!
//! * tar archives (`.tar.xz`, `.tar.gz`, `.tar`) are unpacked through [`archive`],
//!   plain images (`.img`, `.iso`, `.ext4`, `.fat32`) are used in place;
//! * every artifact is verified through [`checksum`] against the sha256
//!   manifests lying on disk next to it, never against fetched ones — a file a
//!   manifest lists must be present, an artifact that no manifest covers is
//!   refused unless the caller skips verification, and a mismatch is always an
//!   error. They cannot tell a tampered download from a good one; they catch
//!   truncated and corrupted builds.
//!
//! Extraction is the expensive step (several gigabytes), so [`prepare`] reuses a
//! previous one unless asked to start over.
//!
//! Writing goes to the privileged helper as a
//! [`FlashPlan`](crate::protocol::flash::FlashPlan) built by [`plan`].
//!
//! The crate is built for a long-lived frontend: it never prints, never
//! initialises logging, never blocks on an async runtime, never touches the
//! network, reports progress through [`Progress`] callbacks, and fails only with
//! an [`Error`] variant a frontend can react to.
//!
//! [`cli`]: https://github.com/deepin-community/deepin-ports-installer

mod error;
mod plan;
mod prepare;
mod progress;
mod source;

pub use error::{Error, Result};
pub use plan::plan;
pub use prepare::{PrepareOptions, Prepared, prepare};
pub use progress::{Phase, Progress};
pub use source::{Source, SourceOptions, remote, resolve};

pub use crate::catalog::{Artifact, Role};
