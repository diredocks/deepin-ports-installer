//! Privileged side of the helper protocol.
//!
//! The sidecar runs as root (through the platform elevation mechanism: `pkexec`
//! on Linux). It binds a loopback WebSocket, announces its address and a per-run
//! token on stdout, and serves the closed set of operations in
//! [`sidecar_protocol::Request`]. It never interprets client-provided paths or
//! command lines.

use std::io::{self, Write};
use std::net::Ipv4Addr;

use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{Duration, timeout};
use tokio_tungstenite::{WebSocketStream, accept_hdr_async};
use tungstenite::Message;
use tungstenite::handshake::server::{
    ErrorResponse, Request as HandshakeRequest, Response as HandshakeResponse,
};
use tungstenite::http::{Response as HttpResponse, StatusCode};

use crate::platform;
use crate::{Error, Result};
use sidecar_protocol::{Handshake, Reply, Request};

/// How long to wait for the client to connect.
const ACCEPT_TIMEOUT: Duration = Duration::from_secs(120);

/// Run the sidecar until the client stops it or disconnects.
pub fn serve() -> Result<()> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(Error::Io)?;

    let listener = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).map_err(Error::Io)?;
    listener.set_nonblocking(true).map_err(Error::Io)?;

    let handshake = Handshake {
        address: listener.local_addr().map_err(Error::Io)?.to_string(),
        token: platform::random_token()?,
    };

    let json = serde_json::to_string(&handshake)?;
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{json}").map_err(Error::Io)?;
    stdout.flush().map_err(Error::Io)?;

    runtime.block_on(serve_on(listener, &handshake.token))
}

/// Serve one authenticated client.
async fn serve_on(listener: std::net::TcpListener, token: &str) -> Result<()> {
    let listener = TcpListener::from_std(listener).map_err(Error::Io)?;
    serve_socket(accept_client(listener, token).await?).await
}

/// Accept one client and complete the WebSocket handshake.
// The tungstenite handshake callback must return a full HTTP response as its
// error type; boxing it would require wrapping the callback.
#[allow(clippy::result_large_err)]
async fn accept_client(listener: TcpListener, token: &str) -> Result<WebSocketStream<TcpStream>> {
    let (stream, _) = timeout(ACCEPT_TIMEOUT, listener.accept())
        .await
        .map_err(|_| Error::Timeout)?
        .map_err(Error::Io)?;

    accept_hdr_async(
        stream,
        |request: &HandshakeRequest, response: HandshakeResponse| {
            if authorized(request, token) {
                Ok(response)
            } else {
                Err(unauthorized())
            }
        },
    )
    .await
    .map_err(|err| Error::Protocol(format!("handshake failed: {err}")))
}

/// Read requests until the client shuts the helper down or disconnects.
async fn serve_socket(mut socket: WebSocketStream<TcpStream>) -> Result<()> {
    while let Some(message) = socket.next().await {
        let text = match message.map_err(Error::WebSocket)? {
            Message::Text(text) => text,
            Message::Close(_) => break,
            _ => continue,
        };

        if handle_request(&mut socket, text.as_str()).await? {
            break;
        }
    }

    Ok(())
}

/// Handle one request, returning whether the helper should stop.
async fn handle_request(socket: &mut WebSocketStream<TcpStream>, text: &str) -> Result<bool> {
    let request: Request = serde_json::from_str(text)
        .map_err(|err| Error::Protocol(format!("invalid request: {err}")))?;

    match request {
        Request::Id => {
            let reply = match platform::id() {
                Ok(output) => Reply::Ok { output },
                Err(err) => Reply::Error {
                    message: err.to_string(),
                },
            };
            let json = serde_json::to_string(&reply)?;
            socket
                .send(Message::Text(json.into()))
                .await
                .map_err(Error::WebSocket)?;
            Ok(false)
        }
        Request::Shutdown => Ok(true),
    }
}

/// Whether the WebSocket request carries the expected token.
fn authorized(request: &HandshakeRequest, token: &str) -> bool {
    let Some(query) = request.uri().query() else {
        return false;
    };

    url::form_urlencoded::parse(query.as_bytes())
        .any(|(key, value)| key == "token" && value == token)
}

/// Build the response used to reject a connection with a bad token.
fn unauthorized() -> ErrorResponse {
    HttpResponse::builder()
        .status(StatusCode::UNAUTHORIZED)
        .body(Some("unauthorized".to_owned()))
        .expect("the unauthorized response is always valid")
}

#[cfg(test)]
mod tests {
    use std::net::{TcpListener as StdListener, TcpStream};
    use std::thread;

    use super::*;

    type Socket = tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>;

    fn start() -> (Handshake, thread::JoinHandle<Result<()>>) {
        let listener = StdListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        listener.set_nonblocking(true).unwrap();

        let handshake = Handshake {
            address: listener.local_addr().unwrap().to_string(),
            token: "test-token".into(),
        };
        let token = handshake.token.clone();

        let server = thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(serve_on(listener, &token))
        });

        (handshake, server)
    }

    fn connect(handshake: &Handshake) -> Socket {
        tungstenite::connect(handshake.url()).unwrap().0
    }

    #[test]
    fn serves_id_and_stops_on_shutdown() {
        let (handshake, server) = start();
        let mut socket = connect(&handshake);

        socket
            .send(Message::Text(
                serde_json::to_string(&Request::Id).unwrap().into(),
            ))
            .unwrap();
        let reply: Reply = serde_json::from_str(socket.read().unwrap().to_text().unwrap()).unwrap();
        match reply {
            Reply::Ok { output } => assert!(output.contains("uid="), "{output}"),
            other => panic!("unexpected reply: {other:?}"),
        }

        socket
            .send(Message::Text(
                serde_json::to_string(&Request::Shutdown).unwrap().into(),
            ))
            .unwrap();
        server.join().unwrap().unwrap();
    }

    #[test]
    fn rejects_a_missing_token() {
        let (handshake, server) = start();
        let url = format!("ws://{}/", handshake.address);

        assert!(tungstenite::connect(url).is_err());
        assert!(server.join().unwrap().is_err());
    }
}
