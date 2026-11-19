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
    let p1 = allocate_fixed(&mut cursor, 2 * MIB_SECTORS, last_usable, "reserved")?;
    let boot_sectors = (500 * MIB_SECTORS).max(sectors_for(boot.size));
    let p2 = allocate_fixed(&mut cursor, boot_sectors, last_usable, "boot")?;
    let p3 = allocate_fixed(&mut cursor, 4 * GIB_SECTORS, last_usable, "swap")?;
    let p4 = allocate_rest(&mut cursor, sectors_for(root.size), last_usable, "root")?;

    Ok(Layout {
        partitions: vec![
            PartitionSpec::new("reserved", TYPE_LINUX_FS, p1),
            PartitionSpec::new("boot", TYPE_LINUX_FS, p2),
            PartitionSpec::new("swap", TYPE_LINUX_SWAP, p3),
            PartitionSpec::new("root", TYPE_LINUX_FS, p4),
        ],
        writes: vec![
            WriteSpec::partition(p2, boot),
            WriteSpec::partition(p4, root),
        ],
    })
}
