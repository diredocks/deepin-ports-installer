//! The job an unprivileged frontend hands to the privileged helper.
//!
//! Carries only what the helper needs to execute the write: the write method,
//! target block device, and ordered artifacts (to be extended with an explicit
//! layout table when partition/offset drivers are implemented) — never raw
//! command lines.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::catalog::{Artifact, FlashVia};

/// What the helper should write, and where.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct FlashPlan {
    /// Write method to use.
    pub via: FlashVia,

    /// Block device node to write to, e.g. `/dev/sdb`.
    pub target: PathBuf,

    /// Artifacts to write, in flash order.
    pub artifacts: Vec<Artifact>,
}
