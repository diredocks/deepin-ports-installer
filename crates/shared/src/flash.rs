//! Frontend-agnostic flash orchestration (`resolve` -> `prepare` -> `plan`).
//!
//! Design constraints: never prints, initializes logging, blocks on an async
//! runtime, or touches the network; reports progress via [`Progress`] and
//! returns typed [`Error`]s.

mod error;
mod plan;
mod prepare;
mod progress;
mod source;

pub use error::{Error, Result};
pub use plan::plan;
pub use prepare::{PrepareOptions, Prepared, prepare};
pub use progress::{Phase, Progress};
pub use source::{Source, SourceOptions, remote, resolve};

pub use crate::catalog::{Artifact, Role};
pub use crate::protocol::FlashPlan;
