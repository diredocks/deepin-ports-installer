use anyhow::Result;

use crate::cli::TargetsArgs;

pub fn run(args: TargetsArgs) -> Result<()> {
    log::debug!("list targets: format={:?}", args.output.format);

    todo!("list local block devices")
}
