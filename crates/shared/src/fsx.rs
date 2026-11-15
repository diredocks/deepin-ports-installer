//! Sync, network-free filesystem primitives shared across the workspace.

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
