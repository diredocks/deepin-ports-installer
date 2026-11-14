use anyhow::{Context, Result};
use clap::Args;

use drives::{Target, TargetList};

use crate::cli::FormatArg;
use crate::render;

#[derive(Debug, Args)]
pub struct TargetsArgs {
    /// Include system disks, read-only and non-removable devices
    #[arg(long)]
    pub all: bool,

    #[command(flatten)]
    pub output: FormatArg,
}

pub fn run(args: TargetsArgs) -> Result<()> {
    let targets = TargetList::scan().context("failed to list local block devices")?;

    log::debug!("found {} local target(s)", targets.targets().len());

    let list = if args.all {
        targets
    } else {
        TargetList::new(targets.into_iter().filter(Target::is_flashable).collect())
    };

    if list.targets().is_empty() {
        if args.all {
            log::info!("no local block devices found");
        } else {
            log::info!(
                "no flashable targets found; use `--all` to include system, read-only and non-removable devices"
            );
        }
    }

    render::targets(&list, args.output.format)
}
