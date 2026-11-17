use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{Context, Result, bail, ensure};
use clap::Args;
use humansize::{BINARY, format_size};
use shared::archive::Archive;
use shared::catalog::{FlashVia, Role, date};
use shared::fetch::download;
use shared::flash::{FlashPlan, PrepareOptions, Prepared, Source, SourceOptions};

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

    /// Write method to use (defaults to the device's preferred method or artifact type)
    #[arg(long, value_enum)]
    pub via: Option<FlashVia>,

    /// Extra file required by the write method (repeatable, e.g. --extra uboot=/path/to/u-boot.bin)
    #[arg(long = "extra", value_name = "ROLE=PATH", value_parser = parse_extra)]
    pub extras: Vec<(Role, PathBuf)>,

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

fn parse_extra(spec: &str) -> std::result::Result<(Role, PathBuf), String> {
    let (role, path) = spec
        .split_once('=')
        .ok_or_else(|| format!("invalid `--extra` value `{spec}`; expected `ROLE=PATH`"))?;

    if path.is_empty() {
        return Err(format!(
            "invalid `--extra` value `{spec}`; missing file path after `=`"
        ));
    }

    let role = Role::from_str(role.trim()).map_err(|err| err.to_string())?;
    if !role.is_extra() {
        return Err(format!(
            "`{role}` is not a user-supplied extra role; expected one of loader, gpt, bootinfo, fsbl, env, opensbi, boot0, uboot"
        ));
    }

    Ok((role, PathBuf::from(path)))
}

pub fn run(args: FlashArgs, quiet: bool) -> Result<()> {
    let options = SourceOptions::from(&args.filters);
    let source =
        shared::flash::resolve(&args.source, &options).context("failed to resolve the flash source")?;

    print_source(&source);
    println!("target: {}", args.target.display());

    let downloaded = source
        .cache_path()
        .context("failed to resolve the download destination")?;

    if args.dry_run {
        print_preparation(&downloaded);
        for (role, path) in &args.extras {
            println!("extra: {} ({role})", path.display());
        }
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
    let result = shared::flash::prepare(&downloaded, &options, |progress| {
        render::prepare_progress(&bar, &progress);
    });
    bar.finish_and_clear();

    let prepared = match result {
        Ok(prepared) => prepared,
        Err(shared::flash::Error::Unverified { path }) => bail!(
            "`{}` is not covered by any checksum manifest; pass `--skip-verify` to accept it",
            path.display()
        ),
        Err(error) => return Err(error).context("failed to prepare the image"),
    };

    print_prepared(&prepared);

    let plan = shared::flash::plan(&source, &prepared, &args.extras, args.via, &args.target)
        .context("failed to plan the flash job")?;
    println!("via: {}", plan.via);
    print_plan_extras(&plan);

    let bar = render::prepare_bar(quiet);
    let result = plan.execute(|progress| {
        render::prepare_progress(&bar, &progress);
    });
    bar.finish_and_clear();
    result.context("failed to write the image to the target device")?;

    log::info!("wrote {}", plan.target.display());

    Ok(())
}

fn print_source(source: &Source) {
    match source {
        Source::Remote { image, .. } => {
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

fn print_plan_extras(plan: &FlashPlan) {
    for artifact in &plan.artifacts {
        if artifact.role.is_extra() {
            println!(
                "extra: {} ({}, {})",
                artifact.path.display(),
                artifact.role,
                format_size(artifact.size, BINARY.decimal_places(1))
            );
        }
    }
}
