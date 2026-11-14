use anyhow::{Context, Result};
use clap::Args;

use catalog::{Arch, DeviceIndex, DeviceQuery};

use crate::cli::FormatArg;
use crate::render;

#[derive(Debug, Args)]
pub struct DevicesArgs {
    /// Only show devices matching this model
    #[arg(value_name = "DEVICE")]
    pub device: Option<String>,

    /// Only match this architecture
    #[arg(short, long, value_enum)]
    pub arch: Option<Arch>,

    /// Only show devices with this SoC
    #[arg(short, long, value_name = "SOC")]
    pub soc: Option<String>,

    #[command(flatten)]
    pub output: FormatArg,
}

pub fn run(args: DevicesArgs) -> Result<()> {
    let index = DeviceIndex::embedded().context("failed to load embedded device index")?;

    let filtered = index.filtered(&DeviceQuery {
        arch: args.arch,
        soc: args.soc.as_deref(),
        device: args.device.as_deref(),
    });

    log::debug!(
        "matched {} of {} device group(s)",
        filtered.groups().len(),
        index.groups().len()
    );

    render::devices(&filtered, args.output.format)
}
