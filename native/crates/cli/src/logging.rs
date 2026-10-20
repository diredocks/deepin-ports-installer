use clap::ValueEnum;

use crate::cli::GlobalArgs;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl From<LogLevel> for log::LevelFilter {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Error => log::LevelFilter::Error,
            LogLevel::Warn => log::LevelFilter::Warn,
            LogLevel::Info => log::LevelFilter::Info,
            LogLevel::Debug => log::LevelFilter::Debug,
            LogLevel::Trace => log::LevelFilter::Trace,
        }
    }
}

/// Initialise `env_logger` according to the global verbosity flags.
pub fn init(global: &GlobalArgs) {
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
