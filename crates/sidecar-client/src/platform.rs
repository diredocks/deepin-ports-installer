//! Platform elevation for the sidecar.
//!
//! The shared client in [`crate::Client`] only needs one per-platform
//! operation: [`spawn`]. Exactly one backend is compiled for the host platform.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub(crate) use linux::spawn;
#[cfg(target_os = "macos")]
pub(crate) use macos::spawn;
#[cfg(target_os = "windows")]
pub(crate) use windows::spawn;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
compile_error!("the sidecar only supports Linux, macOS and Windows");
