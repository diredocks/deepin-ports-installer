//! HTTP client used to talk to the Deepin Ports image repository.

use reqwest::Client;

use crate::{Error, Result};

/// `User-Agent` sent with every request.
const USER_AGENT: &str = concat!("deepin-ports-installer/", env!("CARGO_PKG_VERSION"));

/// Build an HTTP client.
pub fn client() -> Result<Client> {
    // `reqwest` is built without a bundled TLS provider, so install `ring`
    // before the first client is created.
    let _ = rustls::crypto::ring::default_provider().install_default();

    Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(Error::HttpClient)
}

/// Fetch `url` and return its body as text.
pub async fn get(client: &Client, url: &str) -> Result<String> {
    let response = client
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

    response.text().await.map_err(|source| Error::Request {
        url: url.to_owned(),
        source,
    })
}
