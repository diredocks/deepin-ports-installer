//! Unprivileged side of the helper protocol.

use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::Child;

use tungstenite::stream::MaybeTlsStream;

use super::platform;
use crate::fsx::Amount;
use crate::protocol::FlashPlan;
use crate::protocol::{Handshake, Reply, Request};
use crate::sidecar_client::{Error, Result};

const HELPER_ENV: &str = "DEEPIN_PORTS_INSTALLER_HELPER";
const HELPER_NAME: &str = "deepin-ports-installer-helper";
const HELPER_SYSTEM_PATHS: &[&str] = &[
    "/usr/libexec/deepin-ports-installer-helper",
    "/usr/lib/deepin-ports-installer/deepin-ports-installer-helper",
];

/// How to start the sidecar.
#[derive(Debug, Clone)]
pub struct LaunchOptions {
    /// Path to the helper executable.
    pub program: PathBuf,

    /// Elevate through the platform mechanism (for example `pkexec` on Linux)
    /// unless the current process is already root.
    pub elevate: bool,
}

impl LaunchOptions {
    /// Launch `program`, elevating unless the caller is already root.
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            elevate: true,
        }
    }
}

impl Default for LaunchOptions {
    fn default() -> Self {
        Self::new(default_helper_path())
    }
}

fn default_helper_path() -> PathBuf {
    if let Some(path) = std::env::var_os(HELPER_ENV).filter(|value| !value.is_empty()) {
        return PathBuf::from(path);
    }

    let binary_name = format!("{HELPER_NAME}{}", std::env::consts::EXE_SUFFIX);
    let sibling = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(&binary_name)));

    if let Some(path) = sibling.as_ref().filter(|path| path.is_file()) {
        return path.clone();
    }

    if let Some(path) = HELPER_SYSTEM_PATHS
        .iter()
        .map(Path::new)
        .find(|path| path.is_file())
    {
        return path.to_path_buf();
    }

    sibling.unwrap_or_else(|| PathBuf::from(binary_name))
}

/// An open session with a sidecar.
///
/// Dropping the client closes the WebSocket, which makes the helper exit, and
/// reaps the spawned process.
pub struct Client {
    child: Option<Child>,
    socket: tungstenite::WebSocket<MaybeTlsStream<TcpStream>>,
}

impl Client {
    /// Spawn and connect to a sidecar.
    pub fn launch(options: LaunchOptions) -> Result<Self> {
        let mut child = platform::spawn(&options.program, options.elevate)?;

        let handshake = match read_handshake(&mut child) {
            Ok(handshake) => handshake,
            Err(err) => {
                stop(&mut child);
                return Err(err);
            }
        };

        let url = handshake.url();
        let socket = match tungstenite::connect(url) {
            Ok((socket, _response)) => socket,
            Err(err) => {
                stop(&mut child);
                return Err(Error::WebSocket(err));
            }
        };

        Ok(Self {
            child: Some(child),
            socket,
        })
    }

    /// Ask the helper to stop.
    pub fn shutdown(&mut self) -> Result<()> {
        self.send(Request::Shutdown)
    }

    /// Run a flash job through the helper, reporting its progress.
    pub fn flash<F>(&mut self, plan: &FlashPlan, mut progress: F) -> Result<()>
    where
        F: FnMut(Amount),
    {
        self.send(Request::Flash { plan: plan.clone() })?;

        loop {
            match self.recv()? {
                Reply::Progress { amount } => progress(amount),
                Reply::Ok => return Ok(()),
                Reply::Error { message } => return Err(Error::Reported(message)),
            }
        }
    }

    fn send(&mut self, request: Request) -> Result<()> {
        let json = serde_json::to_string(&request)?;
        self.socket
            .send(tungstenite::Message::Text(json.into()))
            .map_err(Error::WebSocket)
    }

    fn recv(&mut self) -> Result<Reply> {
        loop {
            match self.socket.read().map_err(Error::WebSocket)? {
                tungstenite::Message::Text(text) => {
                    return serde_json::from_str::<Reply>(text.as_str())
                        .map_err(|err| Error::Protocol(format!("invalid reply: {err}")));
                }
                tungstenite::Message::Close(_) => {
                    return Err(Error::Protocol(
                        "sidecar closed the connection before replying".to_owned(),
                    ));
                }
                _ => continue,
            }
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        // Closing the socket makes the helper leave its request loop and exit
        // on its own, even when the spawned process is only the elevator.
        let _ = self.socket.close(None);
        let _ = self.socket.flush();
        if let Some(child) = self.child.as_mut() {
            stop(child);
        }
    }
}

/// Read the helper's handshake line.
fn read_handshake(child: &mut Child) -> Result<Handshake> {
    let stdout = child.stdout.take().ok_or(Error::NoHandshake)?;

    for line in BufReader::new(stdout).lines() {
        let line = line.map_err(Error::Io)?;
        if let Ok(handshake) = serde_json::from_str::<Handshake>(&line) {
            return Ok(handshake);
        }
    }

    Err(Error::NoHandshake)
}

/// Reap the spawned process.
fn stop(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, TcpListener};
    use std::thread;

    use crate::catalog::{Artifact, FlashVia, Role};

    use super::*;

    fn sample_plan() -> FlashPlan {
        FlashPlan {
            via: FlashVia::DdWhole,
            target: PathBuf::from("/dev/sdb"),
            artifacts: vec![Artifact {
                path: PathBuf::from("/tmp/disk.img"),
                role: Role::Disk,
                size: 1024,
            }],
        }
    }

    fn spawn_mock_server(replies: Vec<Reply>) -> (Client, thread::JoinHandle<Request>) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let addr = listener.local_addr().unwrap();

        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut ws = tungstenite::accept(stream).unwrap();

            let msg = ws.read().unwrap();
            let request: Request = serde_json::from_str(msg.to_text().unwrap()).unwrap();

            for reply in replies {
                let json = serde_json::to_string(&reply).unwrap();
                ws.send(tungstenite::Message::Text(json.into())).unwrap();
            }
            let _ = ws.close(None);
            let _ = ws.flush();

            request
        });

        let (socket, _) = tungstenite::connect(format!("ws://{addr}/")).unwrap();
        let client = Client {
            child: None,
            socket,
        };
        (client, server)
    }

    #[test]
    fn streams_flash_progress_until_ok() {
        let replies = vec![
            Reply::Progress {
                amount: Amount::new(512, Some(1024)),
            },
            Reply::Progress {
                amount: Amount::new(1024, Some(1024)),
            },
            Reply::Ok,
        ];
        let (mut client, server) = spawn_mock_server(replies);
        let plan = sample_plan();

        let mut reports = Vec::new();
        client.flash(&plan, |amount| reports.push(amount)).unwrap();

        assert_eq!(
            reports,
            [
                Amount::new(512, Some(1024)),
                Amount::new(1024, Some(1024)),
            ]
        );
        assert_eq!(server.join().unwrap(), Request::Flash { plan });
    }

    #[test]
    fn returns_reported_error_from_helper() {
        let replies = vec![
            Reply::Progress {
                amount: Amount::new(256, Some(1024)),
            },
            Reply::Error {
                message: "permission denied".to_owned(),
            },
        ];
        let (mut client, server) = spawn_mock_server(replies);
        let plan = sample_plan();

        let mut reports = Vec::new();
        let err = client
            .flash(&plan, |amount| reports.push(amount))
            .unwrap_err();

        assert_eq!(reports, [Amount::new(256, Some(1024))]);
        assert!(matches!(err, Error::Reported(msg) if msg == "permission denied"));
        assert_eq!(server.join().unwrap(), Request::Flash { plan });
    }
}
