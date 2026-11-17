use shared::catalog::{Artifact, Role};

use super::{Layout, PartitionSpec, TYPE_LINUX_FS, WriteSpec, allocate_rest, sectors_for};
use crate::Result;

pub(super) fn layout(
    artifacts: &[Artifact],
    first_usable: u64,
    last_usable: u64,
) -> Result<Layout> {
    let root = Artifact::by_role(artifacts, Role::Root)?;
    let mut cursor = first_usable;
    let (p1_start, p1_end) =
        allocate_rest(&mut cursor, sectors_for(root.size), last_usable, "root")?;

    Ok(Layout {
        partitions: vec![PartitionSpec {
            name: "root",
            type_guid: TYPE_LINUX_FS,
            starting_lba: p1_start,
            ending_lba: p1_end,
        }],
        writes: vec![WriteSpec {
            sector: p1_start,
            path: root.path.clone(),
            size: root.size,
        }],
    })
}
