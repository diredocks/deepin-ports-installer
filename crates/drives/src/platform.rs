//! Per-platform block device enumeration.
//!
//! Every backend fills the shared [`Target`](crate::Target) model, so only the
//! parts that talk to the operating system live here. Exactly one backend is
//! compiled for the host platform: Linux shells out to `lsblk`, macOS and
//! Windows are scaffolded with `todo!()`.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub(crate) use linux::scan;
#[cfg(target_os = "macos")]
pub(crate) use macos::scan;
#[cfg(target_os = "windows")]
pub(crate) use windows::scan;
