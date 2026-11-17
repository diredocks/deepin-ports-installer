use shared::catalog::{Artifact, Role};

use super::{
    GIB_SECTORS, Layout, MIB_SECTORS, PartitionSpec, TYPE_LINUX_FS, TYPE_LINUX_SWAP, WriteSpec,
    allocate_fixed, allocate_rest, sectors_for,
};
use crate::Result;

pub(super) fn layout(
    artifacts: &[Artifact],
    first_usable: u64,
    last_usable: u64,
) -> Result<Layout> {
    let boot = Artifact::by_role(artifacts, Role::Boot)?;
    let root = Artifact::by_role(artifacts, Role::Root)?;

    let mut cursor = first_usable;
    let (p1_start, p1_end) = allocate_fixed(&mut cursor, 2 * MIB_SECTORS, last_usable, "reserved")?;
    let boot_sectors = (500 * MIB_SECTORS).max(sectors_for(boot.size));
    let (p2_start, p2_end) = allocate_fixed(&mut cursor, boot_sectors, last_usable, "boot")?;
    let (p3_start, p3_end) = allocate_fixed(&mut cursor, 4 * GIB_SECTORS, last_usable, "swap")?;
    let (p4_start, p4_end) =
        allocate_rest(&mut cursor, sectors_for(root.size), last_usable, "root")?;

    Ok(Layout {
        partitions: vec![
            PartitionSpec {
                name: "reserved",
                type_guid: TYPE_LINUX_FS,
                starting_lba: p1_start,
                ending_lba: p1_end,
            },
            PartitionSpec {
                name: "boot",
                type_guid: TYPE_LINUX_FS,
                starting_lba: p2_start,
                ending_lba: p2_end,
            },
            PartitionSpec {
                name: "swap",
                type_guid: TYPE_LINUX_SWAP,
                starting_lba: p3_start,
                ending_lba: p3_end,
            },
            PartitionSpec {
                name: "root",
                type_guid: TYPE_LINUX_FS,
                starting_lba: p4_start,
                ending_lba: p4_end,
            },
        ],
        writes: vec![
            WriteSpec {
                sector: p2_start,
                path: boot.path.clone(),
                size: boot.size,
            },
            WriteSpec {
                sector: p4_start,
                path: root.path.clone(),
                size: root.size,
            },
        ],
    })
}
