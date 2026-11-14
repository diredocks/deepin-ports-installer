//! The job an unprivileged frontend hands to the privileged helper.
//!
//! Writing an image needs root, and the installer does not run as root, so every
//! write is a job the helper executes on the client's behalf. The plan crosses a
//! trust boundary, so it uses the shared model of `catalog` — the write method and
//! artifact roles both sides already know — and adds nothing of its own beyond
//! describing *this* job.
//!
//! The job is declarative on purpose. It names a device group, a write method, a
//! target and the image files, and nothing else: no layout, no offsets, no
//! command line. The helper re-resolves the device group in its own copy of the
//! index, re-enumerates the target and re-checks the artifacts before writing
//! anything, so a plan cannot talk it into writing somewhere the configuration
//! does not allow.

use std::path::PathBuf;

use catalog::{Arch, Artifact, FlashVia};
use serde::{Deserialize, Serialize};

/// What the helper should write, and where.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct FlashPlan {
    /// Architecture of the device group whose layout applies.
    pub arch: Arch,

    /// SoC slug of that device group, e.g. `jh7110`.
    pub soc: String,

    /// Write method to use; the helper checks it against the device group.
    pub via: FlashVia,

    /// Block device node to write to, e.g. `/dev/sdb`.
    pub target: PathBuf,

    /// Artifacts to write, in flash order.
    pub artifacts: Vec<Artifact>,
}
