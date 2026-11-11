use clap::{Args, Parser, Subcommand};

pub use catalog::Arch;

use crate::commands::{DevicesArgs, DownloadArgs, FlashArgs, ImagesArgs, TargetsArgs};
use crate::render::OutputFormat;

/// Install Deepin Ports images on SD cards, USB drives and eMMC.
///
/// `download` fetches an image; `flash` resolves the newest matching image,
/// unpacks and verifies the downloaded copy, then writes it to the given block
/// device. Use `devices`, `images` and `targets` to discover what can be
/// installed where.
#[derive(Debug, Parser)]
#[command(
    name = "deepin-ports-installer-cli",
    version,
    about,
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
    /// Increase logging verbosity (-v: debug, -vv: trace)
    #[arg(
        short,
        long,
        action = clap::ArgAction::Count,
        global = true,
        conflicts_with = "quiet"
    )]
    pub verbose: u8,

    /// Suppress logs; print errors only
    #[arg(short, long, global = true)]
    pub quiet: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// List supported device models
    Devices(DevicesArgs),

    /// List available images
    Images(ImagesArgs),

    /// List local block devices usable as flash targets
    Targets(TargetsArgs),

    /// Download an image to a local file
    Download(DownloadArgs),

    /// Unpack and verify an image, then write it to a block device
    ///
    /// The image must be downloaded first; writing is not implemented yet.
    Flash(FlashArgs),
}

/// Architecture filter.
#[derive(Debug, Args)]
pub struct ArchArg {
    /// Only match this architecture
    #[arg(short, long, value_enum)]
    pub arch: Option<Arch>,
}

/// Image selection shared by `images`, `download` and `flash`.
#[derive(Debug, Args)]
pub struct ImageFilterArgs {
    /// Only match this architecture
    #[arg(short, long, value_enum)]
    pub arch: Option<Arch>,

    /// Only match variants containing this text (e.g. 25-crimson)
    #[arg(long, value_name = "VARIANT")]
    pub variant: Option<String>,

    /// Only match builds published on this day (YYYYMMDD or YYYY-MM-DD)
    #[arg(long, value_name = "DATE")]
    pub date: Option<String>,
}

/// Output format selector.
#[derive(Debug, Args)]
pub struct FormatArg {
    /// Output format
    #[arg(short, long, value_enum, default_value_t)]
    pub format: OutputFormat,
}
