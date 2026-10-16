use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

pub use installer_core::Arch;

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

    /// Identify the currently connected device
    Identify(IdentifyArgs),
}

// --- list ---

#[derive(Debug, Args)]
pub struct ListArgs {
    #[command(subcommand)]
    pub kind: ListKind,
}

#[derive(Debug, Subcommand)]
pub enum ListKind {
    /// List supported device models
    Devices(DevicesArgs),

    /// List available images
    Images(ImagesArgs),

    /// List local block devices usable as flash targets
    Targets(TargetsArgs),
}

#[derive(Debug, Args)]
pub struct DevicesArgs {
    #[command(flatten)]
    pub arch: ArchArg,

    /// Filter by SoC
    #[arg(short, long, value_name = "SOC")]
    pub soc: Option<String>,

    /// Filter by device model
    #[arg(short, long, value_name = "DEVICE")]
    pub device: Option<String>,

    #[command(flatten)]
    pub output: FormatArg,
}

#[derive(Debug, Args)]
pub struct ImagesArgs {
    #[command(flatten)]
    pub arch: ArchArg,

    /// Filter by device model
    #[arg(short, long, value_name = "DEVICE")]
    pub device: Option<String>,

    #[command(flatten)]
    pub variant: VariantArg,

    /// Only show the newest image of each group
    #[arg(long)]
    pub latest: bool,

    #[command(flatten)]
    pub output: FormatArg,
}

#[derive(Debug, Args)]
pub struct TargetsArgs {
    #[command(flatten)]
    pub output: FormatArg,
}

// --- download ---

#[derive(Debug, Args)]
pub struct DownloadArgs {
    /// Device model to resolve an image for
    #[arg(value_name = "DEVICE")]
    pub device: String,

    #[command(flatten)]
    pub arch: ArchArg,

    #[command(flatten)]
    pub variant: VariantArg,

    /// Pick the newest matching image
    #[arg(long, conflicts_with = "date")]
    pub latest: bool,

    /// Pick the image published on this date (YYYYMMDD)
    #[arg(long, value_name = "YYYYMMDD")]
    pub date: Option<String>,

    /// Destination file or directory
    #[arg(short, long, value_name = "PATH")]
    pub output: Option<PathBuf>,

    /// Overwrite an existing file
    #[arg(long)]
    pub force: bool,

    #[command(flatten)]
    pub cache: CacheArg,
}

// --- flash ---

#[derive(Debug, Args)]
pub struct FlashArgs {
    /// Target block device to write to (e.g. /dev/sdX or /dev/mmcblk0)
    #[arg(value_name = "TARGET")]
    pub target: PathBuf,

    #[command(flatten)]
    pub source: ImageSource,

    /// Do not ask for confirmation before writing
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Verify the image checksum after writing
    #[arg(long)]
    pub verify: bool,

    /// Print the planned actions without downloading or writing
    #[arg(long)]
    pub dry_run: bool,

    /// Decompress the image while writing
    #[arg(long)]
    pub decompress: bool,
}

/// How the image to flash is chosen (exactly one source required).
#[derive(Debug, Args)]
#[command(group(
    clap::ArgGroup::new("source")
        .required(true)
        .multiple(false)
        .args(["device", "image", "url"])
))]
pub struct ImageSource {
    /// Resolve and flash the latest image for this device model
    #[arg(short, long, value_name = "DEVICE")]
    pub device: Option<String>,

    /// Use a local image file
    #[arg(long, value_name = "PATH")]
    pub image: Option<PathBuf>,

    /// Use a remote image URL
    #[arg(long, value_name = "URL")]
    pub url: Option<String>,

    #[command(flatten)]
    pub arch: ArchArg,

    #[command(flatten)]
    pub variant: VariantArg,

    #[command(flatten)]
    pub cache: CacheArg,
}

// --- identify ---

#[derive(Debug, Args)]
pub struct IdentifyArgs {
    #[command(flatten)]
    pub output: FormatArg,
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

/// Download cache selector (`--cache-dir`).
#[derive(Debug, Args)]
pub struct CacheArg {
    /// Directory used to cache downloaded images
    #[arg(long, value_name = "DIR", env = "DEEPIN_PORTS_CACHE_DIR")]
    pub cache_dir: Option<PathBuf>,
}

// --- shared value types ---

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    #[default]
    Table,
    Json,
    Toml,
    Plain,
}

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
