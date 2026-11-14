//! Maintainer tooling for the Deepin Ports installer manifests.
//!
//! This is deliberately separate from `deepin-ports-installer-cli`: crawling
//! the image repository is not something end users do, and keeping the crawler
//! (and its dependencies) out of the installer keeps the shipped CLI small.

mod crawl;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "deepin-ports-manifest",
    version,
    about = "Regenerate the images.toml manifest bundled into the Deepin Ports installer"
)]
struct Cli {
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

    let index = tokio::runtime::Runtime::new()
        .context("failed to start async runtime")?
        .block_on(crawl::image_index(cli.concurrency))
        .context("failed to generate image index")?;

    match &cli.output {
        Some(path) => {
            crawl::write_file(&index, path)?;
            eprintln!(
                "wrote {} image(s) to {}",
                index.images().len(),
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
