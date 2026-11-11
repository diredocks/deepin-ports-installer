//! Privileged sidecar for the Deepin Ports installer.
//!
//! The installer spawns this binary through the platform elevation mechanism
//! (`pkexec` on Linux); it announces a loopback WebSocket and serves the closed
//! protocol implemented in `sidecar_protocol`. It is deliberately its own
//! small binary so that the root process contains as little code (and as few
//! dependencies) as possible.

fn main() -> std::process::ExitCode {
    match sidecar::serve() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("deepin-ports-installer-helper: {err}");
            std::process::ExitCode::FAILURE
        }
    }
}
