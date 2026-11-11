//! Fetching remote files into the local cache.
//!
//! [`http`] holds the shared HTTP client, [`paths`] the platform application
//! directories, and [`download`] streams a response body to disk with resume
//! support. The library never owns an async runtime: [`download::download`] is
//! an `async fn` the caller drives.

pub mod download;
pub mod http;
pub mod paths;

mod error;
pub use error::{Error, Result};
