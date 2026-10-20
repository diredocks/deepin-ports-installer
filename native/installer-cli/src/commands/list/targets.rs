use anyhow::{Context, Result};
use clap::Args;

use installer_core::{TargetList, targets::Target};

use crate::cli::FormatArg;
use crate::render;

#[derive(Debug, Args)]
pub struct TargetsArgs {
    /// Include system disks and non-removable devices
    #[arg(short, long)]
    pub all: bool,

    #[command(flatten)]
    pub output: FormatArg,
}

pub fn run(args: TargetsArgs) -> Result<()> {
    let targets = TargetList::scan().context("failed to list local block devices")?;

    log::debug!("found {} local target(s)", targets.len());

    let targets = if args.all {
        targets
    } else {
        filter_flashable(targets)
    };

    render::targets(&targets, args.output.format)
}

/// Keep only removable, non-system targets unless the caller asked for all.
fn filter_flashable(targets: TargetList) -> TargetList {
    let total = targets.len();
    let kept: Vec<Target> = targets
        .into_iter()
        .filter(|target| !target.is_system && target.is_removable)
        .collect();

    if kept.len() != total {
        log::debug!(
            "hid {} system/removable-incompatible target(s); use --all to show them",
            total - kept.len()
        );
    }

    TargetList::new(kept)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(is_system: bool, is_removable: bool) -> Target {
        Target {
            enumerator: "lsblk:json".into(),
            bus_type: "USB".into(),
            bus_version: None,
            device: "/dev/sda".into(),
            device_path: None,
            raw: "/dev/sda".into(),
            description: "Test".into(),
            size: None,
            block_size: 512,
            logical_block_size: 512,
            mountpoints: Vec::new(),
            is_read_only: false,
            is_system,
            is_virtual: false,
            is_removable,
            is_card: None,
            is_scsi: None,
            is_usb: None,
            is_uas: None,
            partition_table: None,
        }
    }

    #[test]
    fn keeps_only_removable_non_system_targets() {
        let list = TargetList::new(vec![
            target(true, false),
            target(false, false),
            target(true, true),
            target(false, true),
        ]);

        let kept = filter_flashable(list);

        assert_eq!(kept.len(), 1);
        assert!(!kept.targets()[0].is_system);
        assert!(kept.targets()[0].is_removable);
    }
}
