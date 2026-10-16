use anyhow::Result;
use comfy_table::{Cell, Table, presets};
use humansize::{BINARY, format_size};
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
            print_table(&["ARCH", "SOC", "PLATFORM", "DEVICES"], &rows);
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
            print_table(&["ARCH", "DEVICE", "VARIANT", "DATE", "SIZE"], &rows);
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
            human_size(image.size),
        ]
    })
}

/// Format a byte count using binary units.
fn human_size(bytes: u64) -> String {
    format_size(bytes, BINARY.decimal_places(1))
}

/// Print a Markdown-style table with a header row.
fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    let mut table = Table::new();
    table.load_preset(presets::ASCII_MARKDOWN);

    table.set_header(headers.iter().map(Cell::new));
    for row in rows {
        table.add_row(row.iter().map(Cell::new));
    }

    println!("{}", table.trim_fmt());
}
