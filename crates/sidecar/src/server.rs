//! Privileged side of the helper protocol.
//!
//! The sidecar runs as root (through the platform elevation mechanism: `pkexec`
//! on Linux). It binds a loopback WebSocket, announces its address and a per-run
//! token on stdout, and serves the closed set of operations in
//! [`shared::protocol::Request`].

use std::fmt::Write as _;
use std::future::Future;
use std::io::{self, Write};
use std::net::Ipv4Addr;
use std::pin::pin;

use futures_util::{SinkExt, StreamExt};
use shared::fsx::Amount;
use shared::protocol::{Handshake, Reply, Request};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{Duration, Instant, timeout};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::server::{
    ErrorResponse, Request as HandshakeRequest, Response as HandshakeResponse,
};
use tokio_tungstenite::tungstenite::http::{Response as HttpResponse, StatusCode};
use tokio_tungstenite::{WebSocketStream, accept_hdr_async};

use crate::driver;
use crate::{Error, Result};

const ACCEPT_TIMEOUT: Duration = Duration::from_secs(120);
const PROGRESS_INTERVAL: Duration = Duration::from_millis(50);

enum Flow {
    Continue,
    Stop,
}

/// Run the sidecar until the client stops it or disconnects.
pub fn serve() -> Result<()> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(Error::Io)?;

    runtime.block_on(async {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .map_err(Error::Io)?;

        let handshake = Handshake {
            address: listener.local_addr().map_err(Error::Io)?.to_string(),
            token: random_token()?,
        };

        let json = serde_json::to_string(&handshake)?;
        let mut stdout = io::stdout().lock();
        writeln!(stdout, "{json}").map_err(Error::Io)?;
        stdout.flush().map_err(Error::Io)?;
        drop(stdout);

        serve_socket(accept_client(listener, &handshake.token).await?).await
    })
}

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

async fn serve_socket(mut socket: WebSocketStream<TcpStream>) -> Result<()> {
    while let Some(message) = socket.next().await {
        let text = match message.map_err(Error::WebSocket)? {
            Message::Text(text) => text,
            Message::Close(_) => break,
            _ => continue,
        };

        if matches!(
            handle_request(&mut socket, text.as_str()).await?,
            Flow::Stop
        ) {
            break;
        }
    }

    Ok(())
}

async fn handle_request(socket: &mut WebSocketStream<TcpStream>, text: &str) -> Result<Flow> {
    let request: Request = serde_json::from_str(text)
        .map_err(|err| Error::Protocol(format!("invalid request: {err}")))?;

    match request {
        Request::Flash { plan } => {
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
        Request::Shutdown => Ok(Flow::Stop),
    }
}

async fn send_reply(socket: &mut WebSocketStream<TcpStream>, reply: &Reply) -> Result<()> {
    let json = serde_json::to_string(reply)?;
    socket
        .send(Message::Text(json.into()))
        .await
        .map_err(Error::WebSocket)
}

async fn drive_flash(
    socket: &mut WebSocketStream<TcpStream>,
    mut rx: tokio::sync::watch::Receiver<Amount>,
    write: impl Future<Output = Result<()>>,
) -> Result<Flow> {
    let mut write = pin!(write);
    let mut progress_open = true;
    let mut pending_progress = false;
    let mut last_sent: Option<Instant> = None;
    let mut throttle = pin!(tokio::time::sleep(Duration::ZERO));

    let outcome = loop {
        tokio::select! {
            result = &mut write => break result,
            changed = rx.changed(), if progress_open && !pending_progress => {
                match changed {
                    Ok(()) => {
                        let now = Instant::now();
                        match last_sent {
                            Some(prev) if now.duration_since(prev) < PROGRESS_INTERVAL => {
                                pending_progress = true;
                                throttle.as_mut().reset(prev + PROGRESS_INTERVAL);
                            }
                            _ => {
                                let amount = *rx.borrow_and_update();
                                send_reply(socket, &Reply::Progress { amount }).await?;
                                last_sent = Some(now);
                            }
                        }
                    }
                    Err(_) => progress_open = false,
                }
            }
            () = &mut throttle, if pending_progress => {
                pending_progress = false;
                let amount = *rx.borrow_and_update();
                send_reply(socket, &Reply::Progress { amount }).await?;
                last_sent = Some(Instant::now());
            }
            incoming = socket.next() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None => return Ok(Flow::Stop),
                    Some(Err(err)) => return Err(Error::WebSocket(err)),
                    Some(Ok(_)) => {}
                }
            }
        }
    };

    if pending_progress || rx.has_changed().unwrap_or(false) {
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
    Ok(Flow::Continue)
}

fn authorized(request: &HandshakeRequest, token: &str) -> bool {
    request.uri().query().is_some_and(|query| {
        query
            .split('&')
            .any(|pair| pair.strip_prefix("token=") == Some(token))
    })
}

fn unauthorized() -> ErrorResponse {
    HttpResponse::builder()
        .status(StatusCode::UNAUTHORIZED)
        .body(Some("unauthorized".to_owned()))
        .expect("the unauthorized response is always valid")
}

fn random_token() -> Result<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes)?;

    let mut token = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(token, "{byte:02x}");
    }
    Ok(token)
}

#[cfg(test)]
mod tests {
    use std::net::TcpStream as StdTcpStream;
    use std::thread;

    use shared::protocol::FlashPlan;
    use tokio_tungstenite::tungstenite;

    use super::*;
    use crate::testutils::TestDir;

    type Socket = tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<StdTcpStream>>;

    fn spawn_server<T, F, Fut>(task: F) -> (Handshake, thread::JoinHandle<Result<T>>)
    where
        T: Send + 'static,
        F: FnOnce(WebSocketStream<TcpStream>) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T>>,
    {
        let (tx, rx) = std::sync::mpsc::channel();
        let server = thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async move {
                    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
                    let handshake = Handshake {
                        address: listener.local_addr().unwrap().to_string(),
                        token: "test-token".into(),
                    };
                    let token = handshake.token.clone();
                    tx.send(handshake).unwrap();
                    let socket = accept_client(listener, &token).await?;
                    task(socket).await
                })
        });

        (rx.recv().unwrap(), server)
    }

    fn start() -> (Handshake, thread::JoinHandle<Result<()>>) {
        spawn_server(serve_socket)
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
        let dir = TestDir::new("server", "flash-ok");
        let image = dir.join("disk.img");
        let target = dir.join("target.bin");
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
        let dir = TestDir::new("server", "flash-err");
        let target = dir.join("target.bin");
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

    #[test]
    fn throttles_burst_progress_and_flushes_final_amount() {
        let (handshake, server) = spawn_server(|mut socket| async move {
            let (tx, rx) = tokio::sync::watch::channel(Amount::default());
            let write = async move {
                for done in 1..=20u64 {
                    let _ = tx.send(Amount::new(done, Some(20)));
                    tokio::task::yield_now().await;
                }
                Ok(())
            };
            drive_flash(&mut socket, rx, write).await
        });

        let mut client = connect(&handshake);
        let mut progress = Vec::new();
        loop {
            match read_reply(&mut client) {
                Reply::Progress { amount } => progress.push(amount),
                Reply::Ok => break,
                other => panic!("unexpected reply: {other:?}"),
            }
        }

        assert_eq!(progress.last(), Some(&Amount::new(20, Some(20))));
        assert!(progress.len() < 20);
        assert!(matches!(server.join().unwrap(), Ok(Flow::Continue)));
    }

    fn run_disconnect_case(close_cleanly: bool) -> (Result<Flow>, bool) {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        struct DropFlag(Arc<AtomicBool>);

        impl Drop for DropFlag {
            fn drop(&mut self) {
                self.0.store(true, Ordering::Relaxed);
            }
        }

        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);

        let (handshake, server) = spawn_server(move |mut socket| async move {
            let (tx, rx) = tokio::sync::watch::channel(Amount::default());
            let write = async move {
                let _guard = DropFlag(worker_cancelled);
                let _ = tx.send(Amount::new(1, Some(2)));
                std::future::pending::<Result<()>>().await
            };
            drive_flash(&mut socket, rx, write).await
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
        assert!(matches!(outcome, Ok(Flow::Stop)));
        assert!(cancelled);
    }

    #[test]
    fn cancels_flash_when_client_drops_connection() {
        let (outcome, cancelled) = run_disconnect_case(false);
        assert!(matches!(outcome, Err(Error::WebSocket(_))));
        assert!(cancelled);
    }
}
