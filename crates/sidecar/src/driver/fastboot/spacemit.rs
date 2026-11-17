use shared::catalog::{Artifact, Role};

use super::{Step, flash_step};
use crate::Result;

pub(super) fn steps(artifacts: &[Artifact]) -> Result<Vec<Step>> {
    let fsbl = Artifact::by_role(artifacts, Role::Fsbl)?;
    let uboot = Artifact::by_role(artifacts, Role::Uboot)?;

    Ok(vec![
        Step::StageAndContinue {
            path: fsbl.path.clone(),
            size: fsbl.size,
        },
        Step::StageAndContinue {
            path: uboot.path.clone(),
            size: uboot.size,
        },
        flash_step(artifacts, "gpt", Role::Gpt)?,
        flash_step(artifacts, "bootinfo", Role::Bootinfo)?,
        Step::Flash {
            partition: "fsbl",
            path: fsbl.path.clone(),
            size: fsbl.size,
        },
        flash_step(artifacts, "env", Role::Env)?,
        flash_step(artifacts, "opensbi", Role::Opensbi)?,
        Step::Flash {
            partition: "uboot",
            path: uboot.path.clone(),
            size: uboot.size,
        },
        flash_step(artifacts, "boot", Role::Boot)?,
        flash_step(artifacts, "root", Role::Root)?,
    ])
}
