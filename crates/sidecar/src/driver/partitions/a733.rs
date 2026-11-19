use shared::catalog::{Artifact, Role};

use super::{Layout, PartitionSpec, TYPE_LINUX_FS, WriteSpec, allocate_rest, sectors_for};
use crate::{Error, Result};

const BOOT0_SECTOR: u64 = 256;
const UBOOT_SECTOR: u64 = 24576;
const ROOT_SECTOR: u64 = 65536;

pub(super) fn layout(
    artifacts: &[Artifact],
    first_usable: u64,
    last_usable: u64,
) -> Result<Layout> {
    let boot0 = Artifact::by_role(artifacts, Role::Boot0)?;
    let uboot = Artifact::by_role(artifacts, Role::Uboot)?;
    let root = Artifact::by_role(artifacts, Role::Root)?;

    if BOOT0_SECTOR < first_usable {
        return Err(Error::InvalidPlan(
            "a733 boot0 sector overlaps the primary GPT header".into(),
        ));
    }
    if sectors_for(boot0.size) > UBOOT_SECTOR - BOOT0_SECTOR {
        return Err(Error::InvalidPlan(
            "a733 boot0 image exceeds the gap before uboot at sector 24576".into(),
        ));
    }
    if sectors_for(uboot.size) > ROOT_SECTOR - UBOOT_SECTOR {
        return Err(Error::InvalidPlan(
            "a733 uboot image exceeds the gap before root partition at sector 65536".into(),
        ));
    }

    let mut cursor = ROOT_SECTOR;
    let p1 = allocate_rest(&mut cursor, sectors_for(root.size), last_usable, "root")?;

    Ok(Layout {
        partitions: vec![PartitionSpec::new("root", TYPE_LINUX_FS, p1)],
        writes: vec![
            WriteSpec::region(BOOT0_SECTOR, UBOOT_SECTOR - BOOT0_SECTOR, boot0),
            WriteSpec::region(UBOOT_SECTOR, ROOT_SECTOR - UBOOT_SECTOR, uboot),
            WriteSpec::partition(p1, root),
        ],
    })
}
