use anyhow::Result;

use crate::cli::{ListArgs, ListKind};

pub fn run(args: ListArgs) -> Result<()> {
    match args.kind {
        ListKind::Devices(filter) => {
            log::debug!(
                "list devices: arch={:?} soc={:?} device={:?} format={:?}",
                filter.arch,
                filter.soc,
                filter.device,
                filter.format
            );
            todo!("list supported devices")
        }
        ListKind::Images(filter) => {
            log::debug!(
                "list images: arch={:?} device={:?} variant={:?} latest={} format={:?}",
                filter.arch,
                filter.device,
                filter.variant,
                filter.latest,
                filter.format
            );
            todo!("list available images")
        }
        ListKind::Targets(filter) => {
            log::debug!("list targets: format={:?}", filter.format);
            todo!("list local block devices")
        }
    }
}
