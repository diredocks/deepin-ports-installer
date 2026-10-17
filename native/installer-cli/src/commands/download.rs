use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use humansize::{BINARY, format_size};
use indicatif::{ProgressBar, ProgressStyle};
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
        "downloading {} for {} (built {}, {}) to {}",
        image.variant,
        args.device,
        format_build_date(&image.date),
        format_size(image.size, BINARY.decimal_places(1)),
        dest.display()
    );

    let bar = download_bar(image.size);
    let mut last = 0;

    let progress = |progress: Progress| {
        if let Some(total) = progress.total {
            bar.set_length(total);
        }
        bar.set_position(progress.downloaded);

        if bar.is_hidden()
            && let Some(total) = progress.total.filter(|total| *total > 0)
        {
            let percent = progress.downloaded.saturating_mul(100) / total;
            if percent >= last + 10 {
                last = percent;
                log::info!("{percent}%");
            }
        }
    };

    tokio::runtime::Runtime::new()
        .context("failed to start async runtime")?
        .block_on(download::download(&image, &dest, args.force, progress))
        .context("failed to download image")?;

    bar.finish_and_clear();

    log::info!("wrote {}", dest.display());

    Ok(())
}

/// Render a build timestamp such as `20250625-030117` as a readable
/// `2025-06-25 03:01:17`, falling back to the original string if it does not
/// match the expected 14-digit form.
fn format_build_date(date: &str) -> String {
    let digits: String = date.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() == 14 {
        format!(
            "{}-{}-{} {}:{}:{}",
            &digits[0..4],
            &digits[4..6],
            &digits[6..8],
            &digits[8..10],
            &digits[10..12],
            &digits[12..14]
        )
    } else {
        date.to_owned()
    }
}

/// A progress bar tracking a download of `total` bytes on stderr.
fn download_bar(total: u64) -> ProgressBar {
    let bar = ProgressBar::new(total);
    bar.set_style(
        ProgressStyle::with_template(
            "{msg} [{bar:40}] {bytes}/{total_bytes} ({percent}%) {bytes_per_sec} ETA {eta}",
        )
        .expect("progress template is valid")
        .progress_chars("#>-"),
    );
    bar.set_message("downloading");
    bar
}

#[cfg(test)]
mod tests {
    use super::format_build_date;

    #[test]
    fn build_dates_are_made_readable() {
        assert_eq!(format_build_date("20250625-030117"), "2025-06-25 03:01:17");
    }

    #[test]
    fn unexpected_dates_are_left_alone() {
        assert_eq!(format_build_date("unknown"), "unknown");
    }
}
