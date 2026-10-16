use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use crate::cli::{CacheArg, VariantArg};

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
    pub variant: VariantArg,

    #[command(flatten)]
    pub cache: CacheArg,
}

pub fn run(args: FlashArgs) -> Result<()> {
    log::debug!(
        "flash: target={:?} source={:?} yes={} verify={} dry_run={} decompress={}",
        args.target,
        args.source,
        args.yes,
        args.verify,
        args.dry_run,
        args.decompress
    );

    if args.dry_run {
        println!("dry run: nothing was written");
    }

    todo!("flash image onto `{}`", args.target.display())
}
