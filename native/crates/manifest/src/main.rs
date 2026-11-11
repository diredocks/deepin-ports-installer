//! Maintainer tooling for the Deepin Ports installer manifests.
//!
//! This is deliberately separate from `deepin-ports-installer-cli`: crawling
//! the image repository is not something end users do, and keeping the crawler
//! (and its dependencies) out of the installer keeps the shipped CLI small.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
mod crawl;

use clap::{Args, Parser, Subcommand};
use crawl::Options;

#[derive(Debug, Parser)]
#[command(
    name = "deepin-ports-manifest",
    version,
    about = "Regenerate the manifests bundled into the Deepin Ports installer",
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Crawl the image repository and write an `images.toml` manifest
    Generate(GenerateArgs),
}

#[derive(Debug, Args)]
struct GenerateArgs {
    /// Write the manifest to this file instead of standard output
    #[arg(short, long, value_name = "PATH")]
    output: Option<PathBuf>,

    /// Maximum number of concurrent HTTP requests
    #[arg(long, value_name = "N", default_value_t = crawl::DEFAULT_CONCURRENCY)]
    concurrency: usize,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Generate(args) => generate_manifest(args),
    }
}

fn generate_manifest(args: GenerateArgs) -> Result<()> {
    let options = Options {
        concurrency: args.concurrency,
    };

    let index = tokio::runtime::Runtime::new()
        .context("failed to start async runtime")?
        .block_on(crawl::image_index(&options))
        .context("failed to generate image index")?;

    match &args.output {
        Some(path) => {
            crawl::write_file(&index, path)?;
            eprintln!(
                "wrote {} image group(s) to {}",
                index.groups().len(),
                path.display()
            );
        }
        None => print!(
            "{}",
            index.to_toml().context("failed to serialize image index")?
        ),
    }

    Ok(())
}
