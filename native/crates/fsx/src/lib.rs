//! Filesystem primitives shared by the installer crates.
//!
//! Every crate in this workspace downloads, unpacks, verifies or writes files,
//! and each one used to carry its own copy of the same few primitives: a
//! temporary `.name.part` sibling moved into place with a single `rename`, a
//! `(done, total)` progress pair, a 1 MiB streaming reader that reports it, a
//! sorted directory listing and the `failed to read \`{path}\`` error pair.
//! They live here so that the behaviour — not just the shape — is the same
//! everywhere.
//!
//! Nothing here is asynchronous, spawns a process or touches the network. The
//! crate depends on `std` and `thiserror` only.

pub mod amount;
pub mod chunked;
pub mod error;
pub mod files;
pub mod temp;

pub use amount::Amount;
pub use chunked::{BUFFER_SIZE, Chunked};
pub use error::{Error, Result};
pub use files::{
    create_parents, files_below, files_in, metadata, open, parent_or_current, read_to_string, write,
};
pub use temp::{Mode, Sibling};
