mod download;
mod flash;
mod generate;
mod list;

pub use download::DownloadArgs;
pub use flash::FlashArgs;
pub use generate::GenerateArgs;
pub use list::ListArgs;

use anyhow::Result;

use crate::cli::{Cli, Command};

/// Dispatch a parsed command line to its handler.
pub fn run(cli: Cli) -> Result<()> {
    log::trace!("parsed command: {:?}", cli.command);

    match cli.command {
        Command::List(args) => list::run(args),
        Command::Download(args) => download::run(args),
        Command::Flash(args) => flash::run(args),
        Command::Generate(args) => generate::run(args),
    }
}
