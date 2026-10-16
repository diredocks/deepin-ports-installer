use anyhow::Result;

use crate::cli::FlashArgs;

pub fn run(args: FlashArgs) -> Result<()> {
    log::debug!(
        "flash: target={:?} source={:?} yes={} verify={} dry_run={} decompress={}",
        args.target,
        args.source,
        args.yes,
        args.verify,
        args.dry_run,
        args.decompress
    );

    if args.dry_run {
        println!("dry run: nothing was written");
    }

    todo!("flash image onto `{}`", args.target.display())
}
