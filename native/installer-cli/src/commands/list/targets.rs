use anyhow::{Context, Result};
use clap::Args;

use installer_core::TargetList;

use crate::cli::FormatArg;
use crate::render;

#[derive(Debug, Args)]
pub struct TargetsArgs {
    #[command(flatten)]
    pub output: FormatArg,
}

pub fn run(args: TargetsArgs) -> Result<()> {
    let targets = TargetList::scan().context("failed to list local block devices")?;

    log::debug!("found {} local target(s)", targets.len());

    render::targets(&targets, args.output.format)
}
