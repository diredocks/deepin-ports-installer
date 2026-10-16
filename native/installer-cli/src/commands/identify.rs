use anyhow::Result;
use clap::Args;

use crate::cli::FormatArg;

#[derive(Debug, Args)]
pub struct IdentifyArgs {
    #[command(flatten)]
    pub output: FormatArg,
}

pub fn run(args: IdentifyArgs) -> Result<()> {
    log::debug!("identify: format={:?}", args.output.format);

    todo!("identify the currently connected device")
}
