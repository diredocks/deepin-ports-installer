use std::io::IsTerminal;

use anyhow::Result;
use base::{DeviceGroup, DeviceIndex, ImageGroup, ImageIndex, Target, TargetList};
use clap::ValueEnum;
use comfy_table::{Cell, ContentArrangement, Table, presets};
use humansize::{BINARY, format_size};
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    #[default]
    Table,
    Json,
    Toml,
}

pub fn devices(index: &DeviceIndex, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => print_document(&index.to_json()?),
        OutputFormat::Toml => print_document(&index.to_toml()?),
        OutputFormat::Table => {
            let rows = index.groups().iter().map(device_row).collect::<Vec<_>>();
            print_table(&["Arch", "SOC", "Platform", "Devices"], &rows);
            Ok(())
        }
    }
}

pub fn images(index: &ImageIndex, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => print_document(&index.to_json()?),
        OutputFormat::Toml => print_document(&index.to_toml()?),
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

pub fn targets(list: &TargetList, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => print_document(&list.to_json()?),
        OutputFormat::Toml => print_document(&list.to_toml()?),
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
            format_size(image.size, BINARY.decimal_places(1)),
        ]
    })
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

/// Print a data document to standard output, ending with a newline.
fn print_document(document: &str) -> Result<()> {
    print!("{document}");
    if !document.ends_with('\n') {
        println!();
    }
    Ok(())
}

/// Print a table, or tab-separated columns when the output is not a terminal.
///
/// Piped output must stay machine readable: no borders, no terminal-width
/// wrapping, one record per line.
fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    if std::io::stdout().is_terminal() {
        print_fancy_table(headers, rows);
    } else {
        print_plain_table(headers, rows);
    }
}

fn print_fancy_table(headers: &[&str], rows: &[Vec<String>]) {
    let mut table = Table::new();
    table.load_style(presets::ASCII_MARKDOWN);
    table.set_content_arrangement(ContentArrangement::Dynamic);

    table.set_header(headers.iter().map(Cell::new));
    for row in rows {
        table.add_row(row.iter().map(Cell::new));
    }

    println!("{}", table.trim_fmt());
}

fn print_plain_table(headers: &[&str], rows: &[Vec<String>]) {
    println!("{}", headers.join("\t"));
    for row in rows {
        println!("{}", row.join("\t"));
    }
}

/// A progress bar tracking a download of `total` bytes on stderr.
pub fn download_bar(total: u64, quiet: bool) -> ProgressBar {
    if quiet {
        return ProgressBar::hidden();
    }

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
