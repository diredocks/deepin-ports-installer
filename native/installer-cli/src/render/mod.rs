mod size;
mod table;

use anyhow::Result;
use installer_core::{DeviceGroup, DeviceIndex, ImageGroup, ImageIndex};

use crate::cli::OutputFormat;

/// Render a device index.
pub fn devices(index: &DeviceIndex, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            print!("{}", index.to_json()?);
            Ok(())
        }
        OutputFormat::Toml => {
            print!("{}", index.to_toml()?);
            Ok(())
        }
        OutputFormat::Table => {
            let rows = index.groups().iter().map(device_row).collect::<Vec<_>>();
            table::print(&["ARCH", "SOC", "PLATFORM", "DEVICES"], &rows);
            Ok(())
        }
    }
}

/// Render an image index.
pub fn images(index: &ImageIndex, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            print!("{}", index.to_json()?);
            Ok(())
        }
        OutputFormat::Toml => {
            print!("{}", index.to_toml()?);
            Ok(())
        }
        OutputFormat::Table => {
            let rows = index
                .groups()
                .iter()
                .flat_map(image_rows)
                .collect::<Vec<_>>();
            table::print(&["ARCH", "DEVICE", "VARIANT", "DATE", "SIZE"], &rows);
            Ok(())
        }
    }
}

fn device_row(group: &DeviceGroup) -> Vec<String> {
    vec![
        group.arch.to_string(),
        group.soc.clone(),
        group.platform.clone(),
        group.devices.join(", "),
    ]
}

fn image_rows(group: &ImageGroup) -> impl Iterator<Item = Vec<String>> + '_ {
    group.images.iter().map(move |image| {
        vec![
            group.arch.to_string(),
            group.device.clone(),
            image.variant.clone(),
            image.date.clone(),
            size::human(image.size),
        ]
    })
}
