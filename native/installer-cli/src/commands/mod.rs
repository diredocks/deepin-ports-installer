pub mod download;
pub mod flash;
pub mod identify;
pub mod list;

use anyhow::Result;

use crate::cli::{Cli, Command};

/// Dispatch a parsed command line to its handler.
pub fn run(cli: Cli) -> Result<()> {
    log::trace!("parsed command: {:?}", cli.command);

    match cli.command {
        Command::List(args) => list::run(args),
        Command::Download(args) => download::run(args),
        Command::Flash(args) => flash::run(args),
        Command::Identify(args) => identify::run(args),
    }
}
