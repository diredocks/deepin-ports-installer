//! HTTP client used to talk to the Deepin Ports image repository.

use std::sync::OnceLock;

use reqwest::Client;

use crate::{Error, Result};

/// `User-Agent` sent with every request.
const USER_AGENT: &str = concat!("deepin-ports-installer/", env!("CARGO_PKG_VERSION"));

/// The process-wide HTTP client, built lazily on first use.
///
/// `reqwest::Client` is internally reference-counted and cheap to share (its
/// connection pool is what makes reuse valuable), so a single instance serves
/// every caller, including concurrent downloads.
static CLIENT: OnceLock<Client> = OnceLock::new();

/// Return the shared HTTP client, building it on first use.
pub fn client() -> Result<&'static Client> {
    if let Some(client) = CLIENT.get() {
        return Ok(client);
    }

    let client = build()?;
    Ok(CLIENT.get_or_init(|| client))
}

/// Build a fresh client, installing the TLS provider it needs.
fn build() -> Result<Client> {
    // `reqwest` is built without a bundled TLS provider, so install `ring`
    // before the first client is created. This is idempotent; the process may
    // already have a provider installed by another component.
    let _ = rustls::crypto::ring::default_provider().install_default();

    Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(Error::HttpClient)
}

/// Fetch `url` with the shared client, returning a response with a success
/// status.
pub async fn response(url: &str) -> Result<reqwest::Response> {
    let response = client()?
        .get(url)
        .send()
        .await
        .map_err(|source| Error::Request {
            url: url.to_owned(),
            source,
        })?;

    let status = response.status();
    if !status.is_success() {
        return Err(Error::HttpStatus {
            url: url.to_owned(),
            status,
        });
    }

    Ok(response)
}

/// Fetch `url` with the shared client and return its body as text.
pub async fn get(url: &str) -> Result<String> {
    response(url)
        .await?
        .text()
        .await
        .map_err(|source| Error::Request {
            url: url.to_owned(),
            source,
        })
}
