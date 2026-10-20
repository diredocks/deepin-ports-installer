mod devices;
mod images;
mod targets;

use anyhow::Result;
use clap::{Args, Subcommand};

use self::{devices::DevicesArgs, images::ImagesArgs, targets::TargetsArgs};

#[derive(Debug, Args)]
pub struct ListArgs {
    #[command(subcommand)]
    pub kind: ListKind,
}

#[derive(Debug, Subcommand)]
pub enum ListKind {
    /// List supported device models
    Devices(DevicesArgs),

    /// List available images
    Images(ImagesArgs),

    /// List local block devices usable as flash targets
    Targets(TargetsArgs),
}

pub fn run(args: ListArgs) -> Result<()> {
    match args.kind {
        ListKind::Devices(args) => devices::run(args),
        ListKind::Images(args) => images::run(args),
        ListKind::Targets(args) => targets::run(args),
    }
}
