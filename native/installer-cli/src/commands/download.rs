use anyhow::Result;

use crate::cli::DownloadArgs;

pub fn run(args: DownloadArgs) -> Result<()> {
    log::debug!(
        "download: device={} arch={:?} variant={:?} latest={} date={:?} output={:?} \
         force={} cache_dir={:?}",
        args.device,
        args.arch.arch,
        args.variant.variant,
        args.latest,
        args.date,
        args.output,
        args.force,
        args.cache.cache_dir
    );

    todo!("download image for device `{}`", args.device)
}
