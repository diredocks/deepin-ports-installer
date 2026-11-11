use std::path::PathBuf;

use ::flash::{Plan, Source, SourceOptions};
use anyhow::{Context, Result, bail};
use catalog::date;
use clap::Args;
use fetch::download;
use humansize::{BINARY, format_size};

use crate::cli::ImageFilterArgs;

#[derive(Debug, Args)]
pub struct FlashArgs {
    /// Device model (e.g. visionfive-2) or a local image file
    ///
    /// A value containing a path separator, or naming an existing file, is
    /// treated as a local image; anything else is resolved as a device model.
    #[arg(value_name = "SOURCE")]
    pub source: String,

    /// Block device to write to (e.g. /dev/sdX or /dev/mmcblk0)
    #[arg(value_name = "TARGET")]
    pub target: PathBuf,

    #[command(flatten)]
    pub filters: ImageFilterArgs,

    /// Print the resolved source and target without writing anything
    #[arg(short = 'n', long)]
    pub dry_run: bool,
}

pub fn run(args: FlashArgs) -> Result<()> {
    if !args.dry_run {
        bail!(
            "writing images to block devices is not implemented yet; re-run with `--dry-run` to inspect the plan"
        );
    }

    let plan = ::flash::plan(
        &args.source,
        &args.target,
        &SourceOptions {
            arch: args.filters.arch,
            variant: args.filters.variant.as_deref(),
            date: args.filters.date.as_deref(),
        },
    )
    .context("failed to resolve the flash source")?;

    print_plan(&plan);

    Ok(())
}

fn print_plan(plan: &Plan) {
    match &plan.source {
        Source::Remote(image) => {
            println!(
                "source: {} (built {}, {})",
                image.variant,
                date::format(&image.date),
                format_size(image.size, BINARY.decimal_places(1))
            );
            println!("  link: {}", image.link);
            if let Ok(path) = download::destination(None, &image.link) {
                println!("  file: {}", path.display());
            }
        }
        Source::Local { path, size } => {
            println!(
                "source: {} ({}, local file)",
                path.display(),
                format_size(*size, BINARY.decimal_places(1))
            );
        }
    }

    println!("target: {}", plan.target.display());
}
