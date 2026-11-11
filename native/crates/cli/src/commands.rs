mod devices;
mod download;
mod flash;
mod images;
mod targets;

pub use devices::DevicesArgs;
pub use download::DownloadArgs;
pub use flash::FlashArgs;
pub use images::ImagesArgs;
pub use targets::TargetsArgs;

use anyhow::Result;

use crate::cli::{Cli, Command};

pub fn run(cli: Cli) -> Result<()> {
    let quiet = cli.global.quiet;

    match cli.command {
        Command::Devices(args) => devices::run(args),
        Command::Images(args) => images::run(args),
        Command::Targets(args) => targets::run(args),
        Command::Download(args) => download::run(args, quiet),
        Command::Flash(args) => flash::run(args, quiet),
    }
}
