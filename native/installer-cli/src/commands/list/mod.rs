mod devices;
mod filter;
mod images;
mod targets;

use anyhow::Result;

use crate::cli::{ListArgs, ListKind};

pub fn run(args: ListArgs) -> Result<()> {
    match args.kind {
        ListKind::Devices(args) => devices::run(args),
        ListKind::Images(args) => images::run(args),
        ListKind::Targets(args) => targets::run(args),
    }
}
