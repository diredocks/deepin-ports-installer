use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use humansize::{BINARY, format_size};
use installer_core::download::{self, Progress, Query};
use installer_core::{DeviceIndex, ImageIndex};

use crate::cli::{ArchArg, VariantArg};

#[derive(Debug, Args)]
pub struct DownloadArgs {
    /// Device model to resolve an image for
    #[arg(value_name = "DEVICE")]
    pub device: String,

    #[command(flatten)]
    pub arch: ArchArg,

    #[command(flatten)]
    pub variant: VariantArg,

    /// Pick the image published on this date (YYYYMMDD); defaults to the newest
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
    let images = ImageIndex::embedded().context("failed to load embedded image index")?;
    let devices = DeviceIndex::embedded().context("failed to load embedded device index")?;

    let query = Query {
        device: &args.device,
        arch: args.arch.arch,
        variant: args.variant.variant.as_deref(),
        date: args.date.as_deref(),
    };

    let image = download::resolve(&images, &devices, &query)
        .context("failed to resolve a downloadable image")?;

    let dest = download::destination(args.output.as_deref(), &image)?;

    log::info!(
        "downloading {} {} ({}) to {}",
        image.variant,
        image.date,
        format_size(image.size, BINARY.decimal_places(1)),
        dest.display()
    );

    let mut last = 0;
    let progress = move |progress: Progress| {
        let Some(total) = progress.total.filter(|total| *total > 0) else {
            return;
        };

        let percent = progress.downloaded.saturating_mul(100) / total;
        if percent >= last + 10 {
            last = percent;
            log::info!("{percent}%");
        }
    };

    tokio::runtime::Runtime::new()
        .context("failed to start async runtime")?
        .block_on(download::download(&image, &dest, args.force, progress))
        .context("failed to download image")?;

    log::info!("wrote {}", dest.display());

    Ok(())
}
