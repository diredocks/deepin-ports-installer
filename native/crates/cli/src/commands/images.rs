use anyhow::{Context, Result};
use clap::Args;

use catalog::{ImageFilter, ImageIndex, ImageQuery};

use crate::cli::{FormatArg, ImageFilterArgs};
use crate::render;

#[derive(Debug, Args)]
pub struct ImagesArgs {
    /// Only show images for this device model
    #[arg(value_name = "DEVICE")]
    pub device: Option<String>,

    #[command(flatten)]
    pub filters: ImageFilterArgs,

    /// Only show the newest image of each group
    #[arg(long)]
    pub latest: bool,

    #[command(flatten)]
    pub output: FormatArg,
}

pub fn run(args: ImagesArgs) -> Result<()> {
    let index = ImageIndex::embedded().context("failed to load embedded image index")?;

    let groups = index.filter(&ImageQuery {
        arch: args.filters.arch,
        device: args.device.as_deref(),
        filter: ImageFilter {
            variant: args.filters.variant.as_deref(),
            date: args.filters.date.as_deref(),
            latest: args.latest,
        },
    });

    log::debug!(
        "matched {} image group(s) of {}",
        groups.len(),
        index.groups().len()
    );

    render::images(&ImageIndex::from(groups), args.output.format)
}
