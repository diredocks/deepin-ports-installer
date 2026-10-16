pub mod download;
pub mod flash;
pub mod identify;
pub mod list;

use anyhow::Result;

use crate::cli::{Cli, Command, GlobalArgs};

/// Initialise `env_logger` according to the global verbosity flags.
pub fn init_logging(global: &GlobalArgs) {
    let level = match (&global.log_level, global.quiet, global.verbose) {
        (Some(level), ..) => (*level).into(),
        (None, true, _) => log::LevelFilter::Error,
        (None, false, 0) => log::LevelFilter::Info,
        (None, false, 1) => log::LevelFilter::Debug,
        (None, false, _) => log::LevelFilter::Trace,
    };

    env_logger::Builder::new()
        .filter_level(level)
        .format_timestamp(None)
        .parse_default_env()
        .init();

    log::trace!("log level set to {level:?}");
}

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
