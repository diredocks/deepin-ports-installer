//! Core library for the Deepin Ports installer.
//!
//! Consolidates the shared data model (`catalog`), filesystem primitives
//! (`fsx`), sidecar wire types (`protocol`), and—under the `full` feature—the
//! download, extraction, verification, drive discovery, flash orchestration,
//! and sidecar client modules.

pub mod catalog;
pub mod fsx;
pub mod protocol;
pub mod targets;

#[cfg(feature = "full")]
pub mod archive;
#[cfg(feature = "full")]
pub mod checksum;
#[cfg(feature = "full")]
pub mod fetch;
#[cfg(feature = "full")]
pub mod flash;
#[cfg(feature = "full")]
pub mod sidecar_client;

#[cfg(all(test, feature = "full"))]
mod testutils;
