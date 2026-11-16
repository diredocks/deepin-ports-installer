//! Privileged side of the helper protocol.
//!
//! The sidecar runs as root (through the platform elevation mechanism: `pkexec`
//! on Linux). It binds a loopback WebSocket, announces its address and a per-run
//! token on stdout, and serves the closed set of operations in
//! [`shared::protocol::Request`].

use std::future::Future;
use std::io::{self, Write};
use std::net::Ipv4Addr;
use std::pin::pin;

use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{Duration, timeout};
use tokio_tungstenite::{WebSocketStream, accept_hdr_async};
use tungstenite::Message;
use tungstenite::handshake::server::{
    ErrorResponse, Request as HandshakeRequest, Response as HandshakeResponse,
};
use tungstenite::http::{Response as HttpResponse, StatusCode};

use crate::driver;
use crate::platform;
use crate::{Error, Result};
use shared::fsx::Amount;
use shared::protocol::FlashPlan;
use shared::protocol::{Handshake, Reply, Request};

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
        Request::Flash { plan } => handle_flash(socket, plan).await,
        Request::Shutdown => Ok(true),
    }
}

async fn send_reply(socket: &mut WebSocketStream<TcpStream>, reply: &Reply) -> Result<()> {
    let json = serde_json::to_string(reply)?;
    socket
        .send(Message::Text(json.into()))
        .await
        .map_err(Error::WebSocket)
}

/// Run a flash job, streaming its progress to the client.
async fn handle_flash(socket: &mut WebSocketStream<TcpStream>, plan: FlashPlan) -> Result<bool> {
    let (tx, rx) = tokio::sync::watch::channel(Amount::default());
    drive_flash(
        socket,
        rx,
        driver::write(&plan, |amount| {
            let _ = tx.send(amount);
        }),
    )
    .await
}

async fn drive_flash(
    socket: &mut WebSocketStream<TcpStream>,
    mut rx: tokio::sync::watch::Receiver<Amount>,
    write: impl Future<Output = Result<()>>,
) -> Result<bool> {
    let mut write = pin!(write);
    let mut progress_open = true;

    let outcome = loop {
        tokio::select! {
            result = &mut write => break result,
            changed = rx.changed(), if progress_open => {
                match changed {
                    Ok(()) => {
                        let amount = *rx.borrow_and_update();
                        send_reply(socket, &Reply::Progress { amount }).await?;
                    }
                    Err(_) => progress_open = false,
                }
            }
            incoming = socket.next() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None => return Ok(true),
                    Some(Err(err)) => return Err(Error::WebSocket(err)),
                    Some(Ok(_)) => {}
                }
            }
        }
    };

    if rx.has_changed().unwrap_or(false) {
        let amount = *rx.borrow_and_update();
        send_reply(socket, &Reply::Progress { amount }).await?;
    }

    let reply = match outcome {
        Ok(()) => Reply::Ok,
        Err(err) => Reply::Error {
            message: err.to_string(),
        },
    };

    send_reply(socket, &reply).await?;
    Ok(false)
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
    fn rejects_a_missing_token() {
        let (handshake, server) = start();
        let url = format!("ws://{}/", handshake.address);

        assert!(tungstenite::connect(url).is_err());
        assert!(server.join().unwrap().is_err());
    }

    struct TestDir {
        path: std::path::PathBuf,
    }

    impl TestDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "deepin-ports-sidecar-server-{}-{name}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("create test directory");
            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn send_request(socket: &mut Socket, request: &Request) {
        socket
            .send(Message::Text(
                serde_json::to_string(request).unwrap().into(),
            ))
            .unwrap();
    }

    fn read_reply(socket: &mut Socket) -> Reply {
        serde_json::from_str(socket.read().unwrap().to_text().unwrap()).unwrap()
    }

    #[test]
    fn streams_flash_progress_and_finishes_with_ok() {
        let dir = TestDir::new("flash-ok");
        let image = dir.path.join("disk.img");
        let target = dir.path.join("target.bin");
        let payload = b"whole-disk-payload";
        std::fs::write(&image, payload).unwrap();
        std::fs::write(&target, vec![0u8; payload.len()]).unwrap();

        let (handshake, server) = start();
        let mut socket = connect(&handshake);

        let plan = FlashPlan {
            via: shared::catalog::FlashVia::DdWhole,
            target: target.clone(),
            artifacts: vec![shared::catalog::Artifact {
                path: image,
                role: shared::catalog::Role::Disk,
                size: payload.len() as u64,
            }],
        };
        send_request(&mut socket, &Request::Flash { plan });

        let mut progress = Vec::new();
        loop {
            match read_reply(&mut socket) {
                Reply::Progress { amount } => progress.push(amount),
                Reply::Ok => break,
                other => panic!("unexpected reply: {other:?}"),
            }
        }

        assert_eq!(
            progress.last(),
            Some(&shared::fsx::Amount::new(
                payload.len() as u64,
                Some(payload.len() as u64)
            ))
        );
        assert_eq!(std::fs::read(&target).unwrap(), payload);

        send_request(&mut socket, &Request::Shutdown);
        server.join().unwrap().unwrap();
    }

    #[test]
    fn returns_error_reply_for_invalid_flash_plan_and_stays_alive() {
        let dir = TestDir::new("flash-err");
        let target = dir.path.join("target.bin");
        std::fs::write(&target, b"placeholder").unwrap();

        let (handshake, server) = start();
        let mut socket = connect(&handshake);

        let plan = FlashPlan {
            via: shared::catalog::FlashVia::DdWhole,
            target,
            artifacts: Vec::new(),
        };
        send_request(&mut socket, &Request::Flash { plan });

        match read_reply(&mut socket) {
            Reply::Error { message } => assert!(!message.is_empty()),
            other => panic!("unexpected reply: {other:?}"),
        }

        send_request(&mut socket, &Request::Shutdown);
        server.join().unwrap().unwrap();
    }

    fn run_disconnect_case(close_cleanly: bool) -> (Result<bool>, bool) {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        struct DropFlag(Arc<AtomicBool>);

        impl Drop for DropFlag {
            fn drop(&mut self) {
                self.0.store(true, Ordering::Relaxed);
            }
        }

        let listener = StdListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let handshake = Handshake {
            address: listener.local_addr().unwrap().to_string(),
            token: "test-token".into(),
        };
        let token = handshake.token.clone();

        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);

        let server = thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async move {
                    let listener = TcpListener::from_std(listener).unwrap();
                    let mut socket = accept_client(listener, &token).await?;
                    let (tx, rx) = tokio::sync::watch::channel(Amount::default());
                    let write = async move {
                        let _guard = DropFlag(worker_cancelled);
                        let _ = tx.send(Amount::new(1, Some(2)));
                        std::future::pending::<Result<()>>().await
                    };
                    drive_flash(&mut socket, rx, write).await
                })
        });

        let mut client = connect(&handshake);
        assert!(matches!(
            read_reply(&mut client),
            Reply::Progress { amount } if amount == Amount::new(1, Some(2))
        ));

        if close_cleanly {
            client.close(None).unwrap();
            client.flush().unwrap();
        } else {
            drop(client);
        }

        let outcome = server.join().unwrap();
        (outcome, cancelled.load(Ordering::Relaxed))
    }

    #[test]
    fn cancels_flash_when_client_closes_cleanly() {
        let (outcome, cancelled) = run_disconnect_case(true);
        assert!(outcome.unwrap());
        assert!(cancelled);
    }

    #[test]
    fn cancels_flash_when_client_drops_connection() {
        let (outcome, cancelled) = run_disconnect_case(false);
        assert!(matches!(outcome, Err(Error::WebSocket(_))));
        assert!(cancelled);
    }
}
