use anyhow::{Context, Result};
use clap::Args;

use catalog::{DeviceIndex, DeviceQuery};

use crate::cli::{ArchArg, FormatArg};
use crate::render;

#[derive(Debug, Args)]
pub struct DevicesArgs {
    /// Only show devices matching this model
    #[arg(value_name = "DEVICE")]
    pub device: Option<String>,

    #[command(flatten)]
    pub arch: ArchArg,

    /// Only show devices with this SoC
    #[arg(short, long, value_name = "SOC")]
    pub soc: Option<String>,

    #[command(flatten)]
    pub output: FormatArg,
}

pub fn run(args: DevicesArgs) -> Result<()> {
    let index = DeviceIndex::embedded().context("failed to load embedded device index")?;

    let groups = index
        .filter(&DeviceQuery {
            arch: args.arch.arch,
            soc: args.soc.as_deref(),
            device: args.device.as_deref(),
        })
        .cloned()
        .collect::<Vec<_>>();

    log::debug!(
        "matched {} device group(s) of {}",
        groups.len(),
        index.groups().len()
    );

    render::devices(&DeviceIndex::from(groups), args.output.format)
}
