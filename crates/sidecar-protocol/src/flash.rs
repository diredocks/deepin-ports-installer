//! The job an unprivileged frontend hands to the privileged helper.
//!
//! Declarative on purpose: a device group, a write method, a target and the image
//! files — never a layout or a command line.

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

    /// Write method to use.
    pub via: FlashVia,

    /// Block device node to write to, e.g. `/dev/sdb`.
    pub target: PathBuf,

    /// Artifacts to write, in flash order.
    pub artifacts: Vec<Artifact>,
}
