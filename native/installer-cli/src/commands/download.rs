use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use crate::cli::VariantArg;

#[derive(Debug, Args)]
pub struct DownloadArgs {
    /// Device model to resolve an image for
    #[arg(value_name = "DEVICE")]
    pub device: String,

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
}

pub fn run(args: DownloadArgs) -> Result<()> {
    log::debug!(
        "download: device={} variant={:?} latest={} date={:?} output={:?} force={}",
        args.device,
        args.variant.variant,
        args.latest,
        args.date,
        args.output,
        args.force
    );

    todo!("download image for device `{}`", args.device)
}
