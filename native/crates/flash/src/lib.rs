//! Orchestration shared by every Deepin Ports installer frontend.
//!
//! The [`cli`] binary and a future graphical installer both drive the same
//! operations from this crate, so the rules for "which image, which device, in
//! which order" live here rather than in either frontend.
//!
//! # Frontend contract
//!
//! The crate is deliberately usable from a long-lived process:
//!
//! * it never prints to stdout or stderr and never initialises logging;
//! * it never creates or blocks on an async runtime;
//! * every failure is a variant of [`Error`], not a formatted string, so a
//!   frontend can react to it (offer a choice, retry, show a hint);
//! * blocking work (spawning `lsblk` or the sidecar) will stay
//!   behind this crate's API rather than in a frontend, and progress will be
//!   reported through callbacks instead of log output.
//!
//! [`cli`]: https://github.com/deepin-community/deepin-ports-installer

mod error;
mod plan;

pub use error::{Error, Result};
pub use plan::{Plan, Source, SourceOptions, plan};
