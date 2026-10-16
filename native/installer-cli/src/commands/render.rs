use anyhow::Result;
use installer_core::{DeviceGroup, ImageGroup};
use serde::Serialize;

use crate::cli::OutputFormat;

/// Wrapper used to emit a device listing as a top-level TOML table.
#[derive(Serialize)]
struct DeviceDocument<'a> {
    devices: &'a [&'a DeviceGroup],
}

/// Render a list of supported device groups.
pub fn devices(groups: &[&DeviceGroup], format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(groups)?),
        OutputFormat::Toml => {
            let document = DeviceDocument { devices: groups };
            print!("{}", toml::to_string_pretty(&document)?);
        }
        OutputFormat::Table => {
            let rows = groups
                .iter()
                .map(|group| {
                    vec![
                        group.arch.to_string(),
                        group.soc.clone(),
                        group.platform.clone(),
                        group.devices.join(", "),
                    ]
                })
                .collect::<Vec<_>>();
            print_table(&["ARCH", "SOC", "PLATFORM", "DEVICES"], &rows);
        }
        OutputFormat::Plain => {
            for group in groups {
                println!(
                    "{}\t{}\t{}\t{}",
                    group.arch,
                    group.soc,
                    group.platform,
                    group.devices.join(",")
                );
            }
        }
    }

    Ok(())
}

/// Render a list of image groups.
pub fn images(groups: &[ImageGroup], format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(groups)?),
        OutputFormat::Toml => print!("{}", images_toml(groups)?),
        OutputFormat::Table => {
            let mut rows = Vec::new();
            for group in groups {
                for image in &group.images {
                    rows.push(vec![
                        group.arch.to_string(),
                        group.device.clone(),
                        image.variant.clone(),
                        image.date.clone(),
                        human_size(image.size),
                    ]);
                }
            }
            print_table(&["ARCH", "DEVICE", "VARIANT", "DATE", "SIZE"], &rows);
        }
        OutputFormat::Plain => {
            for group in groups {
                for image in &group.images {
                    println!(
                        "{}\t{}\t{}\t{}\t{}\t{}",
                        group.arch, group.device, image.variant, image.size, image.date, image.link
                    );
                }
            }
        }
    }

    Ok(())
}

/// Serialize image groups back into the canonical `[[<arch>.<device>]]` layout
/// used by `images.toml`.
fn images_toml(groups: &[ImageGroup]) -> Result<String> {
    let mut root = toml::value::Table::new();

    for group in groups {
        let arch = root
            .entry(group.arch.as_str().to_owned())
            .or_insert_with(|| toml::Value::Table(toml::value::Table::new()));

        let arch_table = arch
            .as_table_mut()
            .expect("architecture entry is always a table");

        let mut images = Vec::with_capacity(group.images.len());
        for image in &group.images {
            images.push(toml::Value::try_from(image)?);
        }

        arch_table.insert(group.device.clone(), toml::Value::Array(images));
    }

    Ok(toml::to_string_pretty(&root)?)
}

/// Print a simple aligned table with a header row.
fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    let header: Vec<String> = headers.iter().map(|cell| (*cell).to_owned()).collect();

    let mut widths: Vec<usize> = header.iter().map(|cell| cell.chars().count()).collect();
    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            if let Some(width) = widths.get_mut(index) {
                *width = (*width).max(cell.chars().count());
            }
        }
    }

    println!("{}", format_row(&header, &widths));
    for row in rows {
        println!("{}", format_row(row, &widths));
    }
}

fn format_row(cells: &[String], widths: &[usize]) -> String {
    let last = cells.len().saturating_sub(1);
    let mut line = String::new();

    for (index, cell) in cells.iter().enumerate() {
        line.push_str(cell);
        if index < last {
            let padding = widths[index].saturating_sub(cell.chars().count());
            line.extend(std::iter::repeat_n(' ', padding + 2));
        }
    }

    line
}

/// Format a byte count using binary units.
fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];

    if bytes < 1024 {
        return format!("{bytes} B");
    }

    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }

    format!("{value:.1} {}", UNITS[unit])
}
