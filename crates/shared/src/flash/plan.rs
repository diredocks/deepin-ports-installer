//! Turning prepared artifacts into the job the privileged helper runs, and executing it.

use std::path::Path;

use crate::catalog::{Artifact, FlashVia, Role};
use crate::flash::{Error, Phase, Prepared, Progress, Result, Source};
use crate::protocol::FlashPlan;
use crate::sidecar_client::{Client, LaunchOptions};

/// Build the [`FlashPlan`] that writes `prepared` to `target`.
///
/// When `via` is provided and `source` lists allowed `flash_via` methods, `via`
/// must be one of them. Otherwise, `via` defaults to the first listed method,
/// or is inferred from a single whole-disk image or ISO.
pub fn plan(
    source: &Source,
    prepared: &Prepared,
    via: Option<FlashVia>,
    target: &Path,
) -> Result<FlashPlan> {
    let (soc, allowed) = match source {
        Source::Remote { image, flash_via } => {
            (Some(image.device.as_str()), flash_via.as_slice())
        }
        Source::Local { .. } => (None, &[][..]),
    };

    let via = select_via(soc, allowed, &prepared.artifacts, via)?;

    Ok(FlashPlan {
        via,
        target: target.to_path_buf(),
        artifacts: prepared.artifacts.clone(),
    })
}

impl FlashPlan {
    /// Launch the privileged sidecar and execute this plan, reporting write progress.
    pub fn execute<F>(&self, mut progress: F) -> Result<()>
    where
        F: FnMut(Progress),
    {
        let mut client = Client::launch(LaunchOptions::default())?;
        let result = client.flash(self, |amount| {
            progress(Progress {
                phase: Phase::Write,
                amount,
            });
        });
        let _ = client.shutdown();
        result.map_err(Into::into)
    }
}

fn select_via(
    soc: Option<&str>,
    allowed: &[FlashVia],
    artifacts: &[Artifact],
    requested: Option<FlashVia>,
) -> Result<FlashVia> {
    if let Some(via) = requested {
        if !allowed.is_empty() && !allowed.contains(&via) {
            return Err(Error::UnsupportedFlashVia {
                via,
                soc: soc.unwrap_or_default().to_owned(),
            });
        }
        return Ok(via);
    }

    if let Some(&via) = allowed.first() {
        return Ok(via);
    }

    match artifacts {
        [artifact] if artifact.role == Role::Disk => Ok(FlashVia::DdWhole),
        [artifact] if artifact.role == Role::Iso => Ok(FlashVia::IsoWrite),
        _ => Err(Error::MissingFlashVia),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{Arch, Image};
    use std::path::PathBuf;

    fn remote_source(flash_via: &[FlashVia]) -> Source {
        Source::Remote {
            image: Image {
                arch: Arch::Riscv,
                device: "jh7110".into(),
                variant: "25-crimson".into(),
                size: 42,
                date: "20250101-000000".into(),
                link: "https://example.com/image.img".into(),
            },
            flash_via: flash_via.to_vec(),
        }
    }

    fn local_source() -> Source {
        Source::Local {
            path: PathBuf::from("/cache/deepin.img"),
            size: 42,
        }
    }

    fn prepared_with(roles: &[Role]) -> Prepared {
        Prepared {
            directory: PathBuf::from("/cache/jh7110"),
            artifacts: roles
                .iter()
                .map(|&role| Artifact {
                    path: PathBuf::from(format!("/cache/jh7110/{role}.bin")),
                    role,
                    size: 42,
                })
                .collect(),
            unverified: Vec::new(),
        }
    }

    #[test]
    fn builds_the_job_from_the_device_group() {
        let source = remote_source(&[FlashVia::DdWhole]);
        let prepared = prepared_with(&[Role::Disk]);
        let plan = plan(
            &source,
            &prepared,
            Some(FlashVia::DdWhole),
            Path::new("/dev/sdb"),
        )
        .unwrap();

        assert_eq!(plan.via, FlashVia::DdWhole);
        assert_eq!(plan.target, PathBuf::from("/dev/sdb"));
        assert_eq!(plan.artifacts, prepared.artifacts);
    }

    #[test]
    fn defaults_to_the_device_groups_preferred_method() {
        let source = remote_source(&[FlashVia::DdPartitions, FlashVia::Fastboot]);
        let prepared = prepared_with(&[Role::Boot, Role::Root]);
        let plan = plan(&source, &prepared, None, Path::new("/dev/sdb")).unwrap();

        assert_eq!(plan.via, FlashVia::DdPartitions);
    }

    #[test]
    fn refuses_a_method_the_device_does_not_list() {
        let source = remote_source(&[FlashVia::DdWhole]);
        let prepared = prepared_with(&[Role::Disk]);
        let error = plan(
            &source,
            &prepared,
            Some(FlashVia::Fastboot),
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

    #[test]
    fn infers_the_write_method_for_local_whole_disk_and_iso() {
        let source = local_source();

        let disk_plan = plan(
            &source,
            &prepared_with(&[Role::Disk]),
            None,
            Path::new("/dev/sdb"),
        )
        .unwrap();
        assert_eq!(disk_plan.via, FlashVia::DdWhole);

        let iso_plan = plan(
            &source,
            &prepared_with(&[Role::Iso]),
            None,
            Path::new("/dev/sdb"),
        )
        .unwrap();
        assert_eq!(iso_plan.via, FlashVia::IsoWrite);
    }

    #[test]
    fn requires_explicit_via_when_local_artifacts_are_ambiguous() {
        let source = local_source();
        let prepared = prepared_with(&[Role::Boot, Role::Root]);

        let error = plan(&source, &prepared, None, Path::new("/dev/sdb")).unwrap_err();
        assert!(matches!(error, Error::MissingFlashVia));

        let plan = plan(
            &source,
            &prepared,
            Some(FlashVia::Fastboot),
            Path::new("/dev/sdb"),
        )
        .unwrap();
        assert_eq!(plan.via, FlashVia::Fastboot);
    }
}
