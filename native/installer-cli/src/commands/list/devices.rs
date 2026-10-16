use anyhow::{Context, Result};

use installer_core::{Arch, DeviceGroup, DeviceIndex};

use super::filter::contains_ignore_case;
use crate::cli::DevicesArgs;
use crate::render;

pub fn run(args: DevicesArgs) -> Result<()> {
    let index = DeviceIndex::embedded().context("failed to load embedded device index")?;
    let format = args.output.format;
    let arch = args.arch.arch;

    let groups: Vec<&DeviceGroup> = index
        .groups()
        .iter()
        .filter(|group| matches(group, arch, args.soc.as_deref(), args.device.as_deref()))
        .collect();

    log::debug!(
        "matched {} device group(s) of {}",
        groups.len(),
        index.len()
    );

    render::devices(&groups, format)
}

/// Whether `group` satisfies every provided filter.
fn matches(
    group: &DeviceGroup,
    arch: Option<Arch>,
    soc: Option<&str>,
    device: Option<&str>,
) -> bool {
    arch.is_none_or(|arch| group.arch == arch)
        && soc.is_none_or(|soc| contains_ignore_case(&group.soc, soc))
        && device.is_none_or(|device| {
            group
                .devices
                .iter()
                .any(|model| contains_ignore_case(model, device))
        })
}
