//! Unprivileged side of the helper protocol.

use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::Child;

use tungstenite::stream::MaybeTlsStream;

use super::platform;
use super::protocol::{Handshake, Reply, Request};
use crate::{Error, Result};

/// How to start the privileged helper.
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

/// An open session with a privileged helper.
///
/// Dropping the client closes the WebSocket, which makes the helper exit, and
/// reaps the spawned process.
pub struct Client {
    child: Child,
    socket: tungstenite::WebSocket<MaybeTlsStream<TcpStream>>,
}

impl Client {
    /// Spawn and connect to a privileged helper.
    pub fn launch(options: LaunchOptions) -> Result<Self> {
        let mut child = platform::spawn_helper(&options.program, options.elevate)?;

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
                return Err(Error::HelperWebSocket(err));
            }
        };

        Ok(Self { child, socket })
    }

    /// Run `id` through the helper and return its stdout.
    pub fn id(&mut self) -> Result<String> {
        self.send(Request::Id)?;

        loop {
            let text = match self.socket.read().map_err(Error::HelperWebSocket)? {
                tungstenite::Message::Text(text) => text,
                _ => continue,
            };

            return match serde_json::from_str::<Reply>(text.as_str())
                .map_err(|err| Error::HelperProtocol(format!("invalid reply: {err}")))?
            {
                Reply::Ok { output } => Ok(output),
                Reply::Error { message } => Err(Error::HelperReported(message)),
            };
        }
    }

    /// Ask the helper to stop.
    pub fn shutdown(&mut self) -> Result<()> {
        self.send(Request::Shutdown)
    }

    fn send(&mut self, request: Request) -> Result<()> {
        let json = serde_json::to_string(&request)?;
        self.socket
            .send(tungstenite::Message::Text(json.into()))
            .map_err(Error::HelperWebSocket)
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        // Closing the socket makes the helper leave its request loop and exit
        // on its own, even when the spawned process is only the elevator.
        let _ = self.socket.close(None);
        let _ = self.socket.flush();
        stop(&mut self.child);
    }
}

/// Read the helper's handshake line.
fn read_handshake(child: &mut Child) -> Result<Handshake> {
    let stdout = child.stdout.take().ok_or(Error::HelperNoHandshake)?;

    for line in BufReader::new(stdout).lines() {
        let line = line.map_err(Error::HelperIo)?;
        if let Ok(handshake) = serde_json::from_str::<Handshake>(&line) {
            return Ok(handshake);
        }
    }

    Err(Error::HelperNoHandshake)
}

/// Reap the spawned process.
fn stop(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}
