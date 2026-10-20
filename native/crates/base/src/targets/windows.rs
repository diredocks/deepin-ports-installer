//! Windows target enumeration (not implemented yet).
//!
//! The intended implementation mirrors `drivelist`'s `src/windows/list.cpp`:
//!
//! 1. enumerate physical drives with `SetupDiGetClassDevs` using the
//!    `GUID_DEVINTERFACE_DISK` interface class;
//! 2. query size, bus type, removable/read-only flags and product strings with
//!    `IOCTL_DISK_GET_DRIVE_GEOMETRY_EX`, `IOCTL_STORAGE_QUERY_PROPERTY` and
//!    `IOCTL_STORAGE_GET_DEVICE_NUMBER`;
//! 3. resolve the mounted volume(s) of each disk to fill `mountpoints`.

use super::TargetList;
use crate::Result;

pub(super) fn scan() -> Result<TargetList> {
    todo!("enumerate block devices on Windows")
}
