//! Turning prepared artifacts into the job the privileged helper runs.
//!
//! Nothing here touches a block device: writing needs root, so the artifacts and
//! the write method of the device group are packed into a
//! [`FlashPlan`] and handed to the helper through `sidecar_client`. The helper
//! re-checks the plan before writing, so this module decides *what* to ask for,
//! never *how* to write it.

use std::path::Path;

use catalog::{DeviceGroup, FlashVia};
use sidecar_protocol::flash::FlashPlan;

use crate::{Error, Prepared, Result};

/// The job that writes `prepared` to `target` on the devices of `group`.
///
/// `via` is one of the methods the device group lists in `flash_via`; a frontend
/// lets the user choose when a device supports several (eMMC through fastboot,
/// SD cards through partitions). Asking for a method the configuration never
/// mentions is a bug in the frontend, not a plan worth executing, and is refused
/// with [`Error::UnsupportedFlashVia`].
pub fn job(
    prepared: &Prepared,
    group: &DeviceGroup,
    via: FlashVia,
    target: &Path,
) -> Result<FlashPlan> {
    if !group.flash_via.contains(&via) {
        return Err(Error::UnsupportedFlashVia {
            via,
            soc: group.soc.clone(),
        });
    }

    Ok(FlashPlan {
        arch: group.arch,
        soc: group.soc.clone(),
        via,
        target: target.to_path_buf(),
        artifacts: prepared.artifacts.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use catalog::{Arch, Artifact, Role};
    use std::path::PathBuf;

    fn group(flash_via: &[FlashVia]) -> DeviceGroup {
        DeviceGroup {
            arch: Arch::Riscv,
            platform: "sifive-u74".into(),
            soc: "jh7110".into(),
            devices: vec!["visionfive-2".into()],
            links: Vec::new(),
            flash_via: flash_via.to_vec(),
        }
    }

    fn prepared() -> Prepared {
        Prepared {
            directory: PathBuf::from("/cache/jh7110"),
            artifacts: vec![Artifact {
                path: PathBuf::from("/cache/jh7110/deepin.img"),
                role: Role::Disk,
                size: 42,
            }],
            unverified: Vec::new(),
        }
    }

    #[test]
    fn builds_the_job_from_the_device_group() {
        let plan = job(
            &prepared(),
            &group(&[FlashVia::DdWhole]),
            FlashVia::DdWhole,
            Path::new("/dev/sdb"),
        )
        .unwrap();

        assert_eq!(plan.arch, Arch::Riscv);
        assert_eq!(plan.soc, "jh7110");
        assert_eq!(plan.via, FlashVia::DdWhole);
        assert_eq!(plan.target, PathBuf::from("/dev/sdb"));
        assert_eq!(plan.artifacts, prepared().artifacts);
    }

    #[test]
    fn refuses_a_method_the_device_does_not_list() {
        let error = job(
            &prepared(),
            &group(&[FlashVia::DdWhole]),
            FlashVia::Fastboot,
            Path::new("/dev/sdb"),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            Error::UnsupportedFlashVia {
                via: FlashVia::Fastboot,
                soc,
            } if soc == "jh7110"
        ));
    }
}
