mod size;
mod table;

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
        OutputFormat::Json => print_json(groups),
        OutputFormat::Toml => print_toml(&DeviceDocument { devices: groups }),
        OutputFormat::Table => {
            let rows = groups
                .iter()
                .map(|group| device_row(group))
                .collect::<Vec<_>>();
            table::print(&["ARCH", "SOC", "PLATFORM", "DEVICES"], &rows);
            Ok(())
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
            Ok(())
        }
    }
}

/// Render a list of image groups.
pub fn images(groups: &[ImageGroup], format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => print_json(groups),
        OutputFormat::Toml => {
            print!("{}", images_toml(groups)?);
            Ok(())
        }
        OutputFormat::Table => {
            let rows = groups.iter().flat_map(image_rows).collect::<Vec<_>>();
            table::print(&["ARCH", "DEVICE", "VARIANT", "DATE", "SIZE"], &rows);
            Ok(())
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
            Ok(())
        }
    }
}

fn print_json<T: Serialize + ?Sized>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn print_toml<T: Serialize + ?Sized>(value: &T) -> Result<()> {
    print!("{}", toml::to_string_pretty(value)?);
    Ok(())
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

        let images = group
            .images
            .iter()
            .map(toml::Value::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        arch_table.insert(group.device.clone(), toml::Value::Array(images));
    }

    Ok(toml::to_string_pretty(&root)?)
}
