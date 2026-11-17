use shared::catalog::{Artifact, Role};

use super::{Step, flash_step};
use crate::Result;

pub(super) fn steps(artifacts: &[Artifact]) -> Result<Vec<Step>> {
    Ok(vec![
        flash_step(artifacts, "boot", Role::Boot)?,
        flash_step(artifacts, "root", Role::Root)?,
    ])
}
