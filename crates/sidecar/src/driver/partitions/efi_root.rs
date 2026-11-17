use shared::catalog::{Artifact, Role};

use super::{Layout, MIB_SECTORS, TYPE_EFI_SYSTEM, layout_two_partitions};
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
        Role::Efi,
        "efi",
        TYPE_EFI_SYSTEM,
        512 * MIB_SECTORS,
    )
}
