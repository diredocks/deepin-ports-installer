//! Platform support for the sidecar.
//!
//! The sidecar only needs one per-platform operation here: generating the
//! per-run authentication token. The unprivileged side (`spawn`, elevation)
//! lives in `shared::sidecar_client`, so the root process never links it.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub(crate) use linux::random_token;
#[cfg(target_os = "macos")]
pub(crate) use macos::random_token;
#[cfg(target_os = "windows")]
pub(crate) use windows::random_token;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
compile_error!("the sidecar only supports Linux, macOS and Windows");
