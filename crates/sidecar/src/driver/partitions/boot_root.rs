use shared::catalog::{Artifact, Role};

use super::{GIB_SECTORS, Layout, TYPE_LINUX_FS, layout_two_partitions};
use crate::Result;

pub(super) fn layout(
    artifacts: &[Artifact],
    first_usable: u64,
    last_usable: u64,
) -> Result<Layout> {
    layout_two_partitions(
        artifacts,
        first_usable,
        last_usable,
        Role::Boot,
        "boot",
        TYPE_LINUX_FS,
        2 * GIB_SECTORS,
    )
}
