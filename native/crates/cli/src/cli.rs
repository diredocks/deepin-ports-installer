use clap::{Args, Parser, Subcommand};

pub use base::Arch;

use crate::commands::{DownloadArgs, FlashArgs, GenerateArgs, ListArgs};
use crate::logging::LogLevel;
use crate::render::OutputFormat;

/// CLI Installer for Deepin Ports images.
///
/// Resolve, download and write a Deepin Ports image to a storage device
/// (SD card, USB drive, eMMC, ...) in a single command.
#[derive(Debug, Parser)]
#[command(
    name = "deepin-ports-installer-cli",
    bin_name = "deepin-ports-installer-cli",
    version,
    about,
    long_about = None,
    propagate_version = true,
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalArgs,

    #[command(subcommand)]
    pub command: Command,
}

/// Options shared by every subcommand.
#[derive(Debug, Args)]
pub struct GlobalArgs {
    /// Increase logging verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true, conflicts_with = "quiet")]
    pub verbose: u8,

    /// Set the log level explicitly
    #[arg(long, value_enum, global = true, conflicts_with_all = ["verbose", "quiet"])]
    pub log_level: Option<LogLevel>,

    /// Suppress all output except errors
    #[arg(short, long, global = true)]
    pub quiet: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// List supported devices or available images
    List(ListArgs),

    /// Download an image to a local file
    Download(DownloadArgs),

    /// Download (if needed) and write an image to a target device
    Flash(FlashArgs),

    /// Regenerate the bundled `images.toml` manifest from the image repository
    Generate(GenerateArgs),
}

// --- reusable argument groups ---

/// Architecture filter (`--arch`).
#[derive(Debug, Args)]
pub struct ArchArg {
    /// Filter by architecture
    #[arg(short, long, value_enum)]
    pub arch: Option<Arch>,
}

/// Image variant filter (`--variant`).
#[derive(Debug, Args)]
pub struct VariantArg {
    /// Filter by image variant (e.g. 25-crimson-preview)
    #[arg(long, value_name = "VARIANT")]
    pub variant: Option<String>,
}

/// Output format selector (`--format`).
#[derive(Debug, Args)]
pub struct FormatArg {
    /// Output format
    #[arg(short, long, value_enum, default_value_t)]
    pub format: OutputFormat,
}
