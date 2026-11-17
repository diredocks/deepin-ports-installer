//! Turning prepared artifacts into the job the privileged helper runs, and executing it.

use std::path::{Path, PathBuf};

use crate::catalog::{Artifact, FlashVia, Role};
use crate::flash::{Error, Phase, Prepared, Progress, Result, Source};
use crate::protocol::FlashPlan;
use crate::sidecar_client::{Client, LaunchOptions};

/// Build the [`FlashPlan`] that writes `prepared` and `extras` to `target`.
///
/// When `via` is provided and `source` lists allowed `flash_via` methods, `via`
/// must be one of them. Otherwise, `via` defaults to the first listed method,
/// or is inferred from a single whole-disk image or ISO.
pub fn plan(
    source: &Source,
    prepared: &Prepared,
    extras: &[(Role, PathBuf)],
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
    let extra_artifacts = resolve_extras(via, extras)?;
    let artifacts = assemble_artifacts(via, &extra_artifacts, &prepared.artifacts)?;

    Ok(FlashPlan {
        via,
        target: target.to_path_buf(),
        artifacts,
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

fn resolve_extras(via: FlashVia, extras: &[(Role, PathBuf)]) -> Result<Vec<Artifact>> {
    let required = via.required_roles();
    let mut resolved = Vec::with_capacity(extras.len());

    for (role, path) in extras {
        if resolved
            .iter()
            .any(|artifact: &Artifact| artifact.role == *role)
        {
            return Err(Error::DuplicateExtra { role: *role });
        }
        if !required.contains(role) {
            return Err(Error::UnexpectedExtra { via, role: *role });
        }

        let metadata = crate::fsx::metadata(path)?;
        if !metadata.is_file() {
            return Err(Error::NotAFile { path: path.clone() });
        }

        resolved.push(Artifact {
            path: path.clone(),
            role: *role,
            size: metadata.len(),
        });
    }

    Ok(resolved)
}

fn assemble_artifacts(
    via: FlashVia,
    extras: &[Artifact],
    prepared: &[Artifact],
) -> Result<Vec<Artifact>> {
    let required = via.required_roles();
    let mut artifacts = Vec::with_capacity(required.len());

    for &role in required {
        let found = extras
            .iter()
            .chain(prepared.iter())
            .find(|artifact| artifact.role == role)
            .cloned();

        match found {
            Some(artifact) => artifacts.push(artifact),
            None if role.is_extra() => return Err(Error::MissingExtra { via, role }),
            None => return Err(Error::MissingArtifact { via, role }),
        }
    }

    Ok(artifacts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{Arch, Image};
    use crate::testutils::Workspace;
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

    fn write_extras(workspace: &Workspace, roles: &[Role]) -> Vec<(Role, PathBuf)> {
        roles
            .iter()
            .map(|&role| {
                let path = workspace.join(format!("{role}.bin"));
                std::fs::write(&path, role.as_str().as_bytes()).unwrap();
                (role, path)
            })
            .collect()
    }

    #[test]
    fn builds_the_job_from_the_device_group() {
        let source = remote_source(&[FlashVia::DdWhole]);
        let prepared = prepared_with(&[Role::Disk]);
        let plan = plan(
            &source,
            &prepared,
            &[],
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
        let source = remote_source(&[FlashVia::DdBootRoot, FlashVia::Fastboot]);
        let prepared = prepared_with(&[Role::Boot, Role::Root]);
        let plan = plan(&source, &prepared, &[], None, Path::new("/dev/sdb")).unwrap();

        assert_eq!(plan.via, FlashVia::DdBootRoot);
        assert_eq!(
            plan.artifacts
                .iter()
                .map(|artifact| artifact.role)
                .collect::<Vec<_>>(),
            [Role::Boot, Role::Root]
        );
    }

    #[test]
    fn refuses_a_method_the_device_does_not_list() {
        let source = remote_source(&[FlashVia::DdWhole]);
        let prepared = prepared_with(&[Role::Disk]);
        let error = plan(
            &source,
            &prepared,
            &[],
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
            &[],
            None,
            Path::new("/dev/sdb"),
        )
        .unwrap();
        assert_eq!(disk_plan.via, FlashVia::DdWhole);

        let iso_plan = plan(
            &source,
            &prepared_with(&[Role::Iso]),
            &[],
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

        let error = plan(&source, &prepared, &[], None, Path::new("/dev/sdb")).unwrap_err();
        assert!(matches!(error, Error::MissingFlashVia));

        let plan = plan(
            &source,
            &prepared,
            &[],
            Some(FlashVia::Fastboot),
            Path::new("/dev/sdb"),
        )
        .unwrap();
        assert_eq!(plan.via, FlashVia::Fastboot);
    }

    #[test]
    fn plans_extra_files_in_flash_order() {
        let root = Workspace::new("deepin-ports-plan", "extras-ok");
        let extras = write_extras(&root, &[Role::Uboot]);
        let built = plan(
            &remote_source(&[FlashVia::FastbootTh1520]),
            &prepared_with(&[Role::Root, Role::Boot]),
            &extras,
            None,
            Path::new("/dev/null"),
        )
        .unwrap();

        let roles: Vec<Role> = built
            .artifacts
            .iter()
            .map(|artifact| artifact.role)
            .collect();
        assert_eq!(roles, [Role::Uboot, Role::Boot, Role::Root]);
        assert_eq!(built.artifacts[0].size, Role::Uboot.as_str().len() as u64);
    }

    #[test]
    fn rejects_invalid_extra_and_artifact_combinations() {
        let root = Workspace::new("deepin-ports-plan", "extras-err");
        let rk = remote_source(&[FlashVia::Rkdeveloptool]);
        let target = Path::new("/dev/null");

        assert!(matches!(
            plan(&rk, &prepared_with(&[Role::Root]), &[], None, target),
            Err(Error::MissingExtra {
                via: FlashVia::Rkdeveloptool,
                role: Role::Loader,
            })
        ));

        let rk_extras = write_extras(&root, &[Role::Loader]);
        assert!(matches!(
            plan(&rk, &prepared_with(&[Role::Boot]), &rk_extras, None, target),
            Err(Error::MissingArtifact {
                via: FlashVia::Rkdeveloptool,
                role: Role::Root,
            })
        ));

        assert!(matches!(
            plan(
                &remote_source(&[FlashVia::DdWhole]),
                &prepared_with(&[Role::Disk]),
                &rk_extras,
                None,
                Path::new("/dev/sdb"),
            ),
            Err(Error::UnexpectedExtra {
                via: FlashVia::DdWhole,
                role: Role::Loader,
            })
        ));

        let loader_path = rk_extras[0].1.clone();
        assert!(matches!(
            plan(
                &rk,
                &prepared_with(&[Role::Root]),
                &[
                    (Role::Loader, loader_path.clone()),
                    (Role::Loader, loader_path),
                ],
                None,
                target,
            ),
            Err(Error::DuplicateExtra { role: Role::Loader })
        ));

        assert!(matches!(
            plan(
                &rk,
                &prepared_with(&[Role::Root]),
                &[(Role::Loader, root.path().to_path_buf())],
                None,
                target,
            ),
            Err(Error::NotAFile { .. })
        ));
    }
}
