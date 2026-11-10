use std::io::Write;

use crate::cli::GlobalArgs;

/// Initialise `env_logger` according to the global verbosity flags.
///
/// `RUST_LOG` still overrides both flags, for users who need per-module
/// filtering.
pub fn init(global: &GlobalArgs) {
    let level = if global.quiet {
        log::LevelFilter::Error
    } else {
        match global.verbose {
            0 => log::LevelFilter::Info,
            1 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace,
        }
    };

    env_logger::Builder::new()
        .filter_level(level)
        .format_timestamp(None)
        .format_target(false)
        .parse_default_env()
        .format(|buf, record| match record.level() {
            log::Level::Error => writeln!(buf, "error: {}", record.args()),
            log::Level::Warn => writeln!(buf, "warning: {}", record.args()),
            _ => writeln!(buf, "{}", record.args()),
        })
        .init();
}
