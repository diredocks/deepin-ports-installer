//! macOS target enumeration (not implemented yet).
//!
//! The intended implementation mirrors `drivelist`'s `src/darwin/list.mm`:
//!
//! 1. enumerate `IOMedia` objects (`IOMediaBSDClient`) to discover disks and
//!    their partitions;
//! 2. read properties such as size, block size, removal and read-only flags
//!    from the IORegistry;
//! 3. collect mount points via `DiskArbitration`, then merge APFS volumes into
//!    their physical store as `drivelist` does in `lib/index.ts`.

use super::TargetList;
use crate::Result;

pub(super) fn scan() -> Result<TargetList> {
    todo!("enumerate block devices on macOS")
}
