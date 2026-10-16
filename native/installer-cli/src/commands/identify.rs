use anyhow::Result;

use crate::cli::IdentifyArgs;

pub fn run(args: IdentifyArgs) -> Result<()> {
    log::debug!("identify: format={:?}", args.output.format);

    todo!("identify the currently connected device")
}
