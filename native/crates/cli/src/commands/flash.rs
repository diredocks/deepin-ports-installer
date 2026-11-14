use std::path::{Path, PathBuf};

use ::flash::{Plan, PrepareOptions, Prepared, Source, SourceOptions};
use anyhow::{Context, Result, bail, ensure};
use archive::Archive;
use catalog::date;
use clap::Args;
use fetch::download;
use humansize::{BINARY, format_size};

use crate::cli::ImageFilterArgs;
use crate::render;

#[derive(Debug, Args)]
pub struct FlashArgs {
    /// Device model (e.g. visionfive-2) or a local image or archive
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

    /// Unpack the archive again even when an extraction is cached
    #[arg(short, long)]
    pub force: bool,

    /// Skip checksum verification and take the image as it is (e.g. an ISO)
    #[arg(long)]
    pub skip_verify: bool,

    /// Print the resolved source, target and extraction directory without preparing anything
    #[arg(short = 'n', long)]
    pub dry_run: bool,
}

pub fn run(args: FlashArgs, quiet: bool) -> Result<()> {
    let options = SourceOptions::from(&args.filters);
    let plan = ::flash::plan(&args.source, &args.target, &options)
        .context("failed to resolve the flash source")?;

    print_plan(&plan);

    let downloaded = archive_path(&plan)?;

    if args.dry_run {
        print_preparation(&downloaded);
        return Ok(());
    }

    ensure!(
        downloaded.is_file(),
        "{} has not been downloaded yet; run `deepin-ports-installer-cli download {}` first",
        downloaded.display(),
        args.source
    );

    let options = PrepareOptions {
        force: args.force,
        skip_verify: args.skip_verify,
    };

    log::info!("preparing {}", downloaded.display());

    let bar = render::prepare_bar(quiet);
    let result = ::flash::prepare(&downloaded, &options, |progress| {
        render::prepare_progress(&bar, &progress);
    });
    bar.finish_and_clear();

    let prepared = match result {
        Ok(prepared) => prepared,
        Err(::flash::Error::Unverified { path }) => bail!(
            "`{}` is not covered by any checksum manifest; pass `--skip-verify` to accept it",
            path.display()
        ),
        Err(error) => return Err(error).context("failed to prepare the image"),
    };

    print_prepared(&prepared);

    todo!(
        "writing images to block devices is not implemented yet; the artifacts are prepared in {}",
        prepared.directory.display()
    )
}

/// Where the archive the plan names is, or will be, cached.
fn archive_path(plan: &Plan) -> Result<PathBuf> {
    match &plan.source {
        Source::Remote(image) => download::destination(None, &image.link)
            .context("failed to resolve the download destination"),
        Source::Local { path, .. } => Ok(path.clone()),
    }
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

fn print_preparation(downloaded: &Path) {
    match Archive::new(downloaded) {
        Some(archive) => println!("extract: {}", archive.extraction_dir().display()),
        None => println!("extract: none (the image is not an archive)"),
    }
}

fn print_prepared(prepared: &Prepared) {
    for artifact in &prepared.artifacts {
        let checksum = if prepared.unverified.contains(&artifact.path) {
            "unverified"
        } else {
            "verified sha256"
        };

        println!(
            "artifact: {} ({}, {}, {checksum})",
            artifact.path.display(),
            artifact.role,
            format_size(artifact.size, BINARY.decimal_places(1))
        );
    }
}
