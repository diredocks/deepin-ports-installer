mod cli;
mod commands;
mod logging;
mod render;

use std::process::ExitCode;

use clap::Parser;

use crate::cli::Cli;

fn main() -> ExitCode {
    restore_sigpipe();

    let cli = Cli::parse();
    logging::init(&cli.global);

    match commands::run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            log::error!("{err:#}");
            ExitCode::FAILURE
        }
    }
}

/// Restore the default `SIGPIPE` disposition on Unix.
///
/// The Rust runtime ignores `SIGPIPE`, which turns `... | head` into a panic
/// on `EPIPE`. Restoring the default makes the process terminate quietly, as
/// every classic Unix filter does.
#[cfg(unix)]
fn restore_sigpipe() {
    // SAFETY: `signal` is called once at startup, before any other thread is
    // spawned, and only changes the disposition of `SIGPIPE`.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_sigpipe() {}
