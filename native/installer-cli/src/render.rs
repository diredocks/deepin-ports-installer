use anyhow::Result;
use clap::ValueEnum;
use comfy_table::{Cell, ContentArrangement, Table, presets};
use humansize::{BINARY, format_size};
use indicatif::{ProgressBar, ProgressStyle};
use installer_core::{DeviceGroup, DeviceIndex, ImageGroup, ImageIndex, Target, TargetList};

/// Output format selector (`--format`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    #[default]
    Table,
    Json,
    Toml,
}

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
            print_table(&["Arch", "SOC", "Platform", "Devices"], &rows);
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
            print_table(&["Arch", "Device", "Variant", "Date", "Size"], &rows);
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

/// Render a target list.
pub fn targets(list: &TargetList, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            print!("{}", list.to_json()?);
            Ok(())
        }
        OutputFormat::Toml => {
            print!("{}", list.to_toml()?);
            Ok(())
        }
        OutputFormat::Table => {
            let rows = list.targets().iter().map(target_row).collect::<Vec<_>>();
            print_table(
                &[
                    "Device",
                    "Size",
                    "Bus",
                    "Table",
                    "Removable",
                    "System",
                    "Read-only",
                    "Description",
                    "Mountpoints",
                ],
                &rows,
            );
            Ok(())
        }
    }
}

fn target_row(target: &Target) -> Vec<String> {
    vec![
        target.device.display().to_string(),
        target
            .size
            .map(|size| format_size(size, BINARY.decimal_places(1)))
            .unwrap_or_else(|| "-".to_owned()),
        target.bus_type.to_string(),
        target
            .partition_table
            .map(|table| table.to_string())
            .unwrap_or_else(|| "-".to_owned()),
        yes_no(target.is_removable),
        yes_no(target.is_system),
        yes_no(target.is_read_only),
        target.description.clone(),
        if target.mountpoints.is_empty() {
            "-".to_owned()
        } else {
            target
                .mountpoints
                .iter()
                .map(|mountpoint| mountpoint.path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        },
    ]
}

fn yes_no(value: bool) -> String {
    if value { "yes" } else { "no" }.to_owned()
}

fn image_rows(group: &ImageGroup) -> impl Iterator<Item = Vec<String>> + '_ {
    group.images.iter().map(move |image| {
        vec![
            group.arch.to_string(),
            group.device.clone(),
            image.variant.clone(),
            image.date.clone(),
            format_size(image.size, BINARY.decimal_places(1)),
        ]
    })
}

/// Print a Markdown-style table with a header row.
fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    let mut table = Table::new();
    table.load_style(presets::ASCII_MARKDOWN);
    table.set_content_arrangement(ContentArrangement::Dynamic);

    table.set_header(headers.iter().map(Cell::new));
    for row in rows {
        table.add_row(row.iter().map(Cell::new));
    }

    println!("{}", table.trim_fmt());
}

/// A progress bar tracking a download of `total` bytes on stderr.
pub fn download_bar(total: u64) -> ProgressBar {
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
