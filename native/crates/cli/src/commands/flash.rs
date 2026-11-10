use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use base::date;
use base::download;
use base::{DeviceIndex, Image, ImageIndex, Query};
use clap::Args;
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

/// What `flash` would write, once writing is implemented.
enum Source {
    /// An image resolved from the bundled index; it may need downloading.
    Remote(Image),

    /// A local image file named by the caller.
    Local { path: PathBuf, size: u64 },
}

pub fn run(args: FlashArgs) -> Result<()> {
    if !args.dry_run {
        bail!(
            "writing images to block devices is not implemented yet; re-run with `--dry-run` to inspect the plan"
        );
    }

    let source = resolve_source(&args)?;
    print_plan(&source, &args.target);

    Ok(())
}

fn resolve_source(args: &FlashArgs) -> Result<Source> {
    if looks_like_file(&args.source) {
        if args.filters.arch.is_some()
            || args.filters.variant.is_some()
            || args.filters.date.is_some()
        {
            bail!("`--arch`, `--variant` and `--date` only apply when SOURCE is a device model");
        }

        let path = PathBuf::from(&args.source);
        let metadata = std::fs::metadata(&path)
            .with_context(|| format!("failed to read image `{}`", path.display()))?;

        if !metadata.is_file() {
            bail!("`{}` is not a regular file", path.display());
        }

        return Ok(Source::Local {
            path,
            size: metadata.len(),
        });
    }

    let images = ImageIndex::embedded().context("failed to load embedded image index")?;
    let devices = DeviceIndex::embedded().context("failed to load embedded device index")?;

    let query = Query {
        device: &args.source,
        arch: args.filters.arch,
        variant: args.filters.variant.as_deref(),
        date: args.filters.date.as_deref(),
    };

    let image = images
        .resolve(&devices, &query)
        .context("failed to resolve an image for this device")?;

    Ok(Source::Remote(image))
}

/// Whether `source` names a local image rather than a device model.
fn looks_like_file(source: &str) -> bool {
    source.chars().any(std::path::is_separator) || Path::new(source).is_file()
}

fn print_plan(source: &Source, target: &Path) {
    match source {
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

    println!("target: {}", target.display());
}
