use anyhow::{Context, Result};
use clap::Args;

use base::text::contains_ignore_case;
use base::{Image, ImageGroup, ImageIndex, date};

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

    let groups: Vec<ImageGroup> = index
        .groups()
        .iter()
        .filter(|group| {
            args.filters.arch.is_none_or(|arch| group.arch == arch)
                && args
                    .device
                    .as_deref()
                    .is_none_or(|device| contains_ignore_case(&group.device, device))
        })
        .filter_map(|group| select_images(group, &args.filters, args.latest))
        .collect();

    log::debug!("matched {} image group(s) of {}", groups.len(), index.len());

    render::images(&ImageIndex::from(groups), args.output.format)
}

/// Return `group` with its images filtered; `None` when no image matches.
fn select_images(
    group: &ImageGroup,
    filters: &ImageFilterArgs,
    latest: bool,
) -> Option<ImageGroup> {
    let mut images: Vec<Image> = group
        .images
        .iter()
        .filter(|image| {
            filters
                .variant
                .as_deref()
                .is_none_or(|variant| contains_ignore_case(&image.variant, variant))
                && filters
                    .date
                    .as_deref()
                    .is_none_or(|day| date::is_on_day(&image.date, day))
        })
        .cloned()
        .collect();

    if images.is_empty() {
        return None;
    }

    if latest {
        let newest = images
            .iter()
            .map(|image| image.date.as_str())
            .max()
            .unwrap_or_default()
            .to_owned();
        images.retain(|image| image.date == newest);
    }

    Some(ImageGroup {
        arch: group.arch,
        device: group.device.clone(),
        images,
    })
}
