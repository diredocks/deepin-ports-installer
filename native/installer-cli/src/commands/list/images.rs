use anyhow::{Context, Result};

use installer_core::{Image, ImageGroup, ImageIndex};

use super::filter::contains_ignore_case;
use crate::cli::ImagesArgs;
use crate::render;

pub fn run(args: ImagesArgs) -> Result<()> {
    let index = ImageIndex::embedded().context("failed to load embedded image index")?;
    let format = args.output.format;
    let arch = args.arch.arch;

    let groups: Vec<ImageGroup> = index
        .groups()
        .iter()
        .filter(|group| arch.is_none_or(|arch| group.arch == arch))
        .filter(|group| {
            args.device
                .as_deref()
                .is_none_or(|device| contains_ignore_case(&group.device, device))
        })
        .filter_map(|group| select_images(group, args.variant.variant.as_deref(), args.latest))
        .collect();

    log::debug!("matched {} image group(s) of {}", groups.len(), index.len());

    render::images(&groups, format)
}

/// Return `group` with its images filtered; `None` when no image matches.
fn select_images(group: &ImageGroup, variant: Option<&str>, latest: bool) -> Option<ImageGroup> {
    let mut images: Vec<Image> = group
        .images
        .iter()
        .filter(|image| variant.is_none_or(|variant| contains_ignore_case(&image.variant, variant)))
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
