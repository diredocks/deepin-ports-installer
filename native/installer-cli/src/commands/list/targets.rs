use anyhow::Result;
use clap::Args;

use crate::cli::FormatArg;

#[derive(Debug, Args)]
pub struct TargetsArgs {
    #[command(flatten)]
    pub output: FormatArg,
}

pub fn run(args: TargetsArgs) -> Result<()> {
    log::debug!("list targets: format={:?}", args.output.format);

    todo!("list local block devices")
}
