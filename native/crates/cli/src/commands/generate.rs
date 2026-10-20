use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;

use base::generate::{self, Options};

#[derive(Debug, Args)]
pub struct GenerateArgs {
    /// Base URL of the Deepin Ports image repository
    #[arg(long, value_name = "URL", default_value = generate::DEFAULT_BASE)]
    pub base: String,

    /// Write the manifest to this file instead of standard output
    #[arg(short, long, value_name = "PATH")]
    pub output: Option<PathBuf>,

    /// Maximum number of concurrent HTTP requests
    #[arg(long, value_name = "N", default_value_t = generate::DEFAULT_CONCURRENCY)]
    pub concurrency: usize,
}

pub fn run(args: GenerateArgs) -> Result<()> {
    let options = Options {
        base: args.base,
        concurrency: args.concurrency,
    };

    let index = tokio::runtime::Runtime::new()
        .context("failed to start async runtime")?
        .block_on(generate::image_index(&options))
        .context("failed to generate image index")?;

    match &args.output {
        Some(path) => {
            generate::write_file(&index, path)?;
            log::info!("wrote {} image group(s) to {}", index.len(), path.display());
        }
        None => print!(
            "{}",
            index.to_toml().context("failed to serialize image index")?
        ),
    }

    Ok(())
}
