use anyhow::{Context, Result};

use installer_core::{DeviceGroup, DeviceIndex, Image, ImageGroup, ImageIndex};

use crate::cli::{DevicesFilter, ImagesFilter, ListArgs, ListKind};
use crate::commands::render;

pub fn run(args: ListArgs) -> Result<()> {
    match args.kind {
        ListKind::Devices(filter) => devices(filter),
        ListKind::Images(filter) => images(filter),
        ListKind::Targets(filter) => {
            log::debug!("list targets: format={:?}", filter.format);
            todo!("list local block devices")
        }
    }
}

fn devices(filter: DevicesFilter) -> Result<()> {
    let index = DeviceIndex::embedded().context("failed to load embedded device index")?;

    let groups: Vec<&DeviceGroup> = index
        .groups()
        .iter()
        .filter(|group| filter.arch.is_none_or(|arch| group.arch == arch))
        .filter(|group| {
            filter
                .soc
                .as_deref()
                .is_none_or(|soc| contains_ignore_case(&group.soc, soc))
        })
        .filter(|group| {
            filter.device.as_deref().is_none_or(|device| {
                group
                    .devices
                    .iter()
                    .any(|model| contains_ignore_case(model, device))
            })
        })
        .collect();

    log::debug!(
        "matched {} device group(s) of {}",
        groups.len(),
        index.len()
    );

    render::devices(&groups, filter.format)
}

fn images(filter: ImagesFilter) -> Result<()> {
    let index = ImageIndex::embedded().context("failed to load embedded image index")?;

    let mut groups = Vec::new();

    for group in index.groups() {
        if filter.arch.is_some_and(|arch| group.arch != arch) {
            continue;
        }
        if filter
            .device
            .as_deref()
            .is_some_and(|device| !contains_ignore_case(&group.device, device))
        {
            continue;
        }

        let mut images: Vec<Image> = group
            .images
            .iter()
            .filter(|image| {
                filter
                    .variant
                    .as_deref()
                    .is_none_or(|variant| contains_ignore_case(&image.variant, variant))
            })
            .cloned()
            .collect();

        if images.is_empty() {
            continue;
        }

        if filter.latest {
            let newest = images
                .iter()
                .map(|image| image.date.as_str())
                .max()
                .unwrap_or_default()
                .to_owned();
            images.retain(|image| image.date == newest);
        }

        groups.push(ImageGroup {
            arch: group.arch,
            device: group.device.clone(),
            images,
        });
    }

    log::debug!("matched {} image group(s) of {}", groups.len(), index.len());

    render::images(&groups, filter.format)
}

/// Case-insensitive substring match used by the `--soc`/`--device`/`--variant`
/// filters.
fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    haystack.to_lowercase().contains(&needle.to_lowercase())
}
