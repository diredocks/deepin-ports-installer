use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use humansize::{BINARY, format_size};
use shared::catalog::date;
use shared::fetch::download::{self, Progress};
use shared::flash::{self, Source, SourceOptions};

use crate::cli::ImageFilterArgs;
use crate::render;

#[derive(Debug, Args)]
pub struct DownloadArgs {
    /// Device model to resolve an image for
    #[arg(value_name = "DEVICE")]
    pub device: String,

    #[command(flatten)]
    pub filters: ImageFilterArgs,

    /// Destination file or directory [default: the cache directory]
    #[arg(short, long, value_name = "PATH")]
    pub output: Option<PathBuf>,

    /// Overwrite an existing file
    #[arg(long)]
    pub force: bool,
}

pub fn run(args: DownloadArgs, quiet: bool) -> Result<()> {
    let options = SourceOptions::from(&args.filters);
    let Source::Remote { image, .. } = flash::remote(&args.device, &options)
        .context("failed to resolve a downloadable image")?
    else {
        unreachable!("flash::remote only returns Source::Remote");
    };

    let dest = download::destination(args.output.as_deref(), &image.link)?;

    log::info!(
        "downloading {} for {} (built {}, {}) to {}",
        image.variant,
        args.device,
        date::format(&image.date),
        format_size(image.size, BINARY.decimal_places(1)),
        dest.display()
    );

    let bar = render::download_bar(image.size, quiet);
    let mut last = 0;

    let progress = |progress: Progress| {
        if let Some(total) = progress.total {
            bar.set_length(total);
        }
        bar.set_position(progress.done);

        if bar.is_hidden()
            && let Some(percent) = progress.percent()
            && percent >= last + 10
        {
            last = percent;
            log::info!("{percent}%");
        }
    };

    tokio::runtime::Runtime::new()
        .context("failed to start async runtime")?
        .block_on(download::download(
            &image.link,
            image.size,
            &dest,
            args.force,
            progress,
        ))
        .context("failed to download image")?;

    bar.finish_and_clear();
    log::info!("wrote {}", dest.display());

    Ok(())
}
