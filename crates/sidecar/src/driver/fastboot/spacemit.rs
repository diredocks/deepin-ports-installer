use std::path::Path;

use shared::catalog::{Artifact, Role};

use super::{Step, flash_step, stage_step};
use crate::Result;

const PARTITIONS_MTD: &str = r#"{
  "version": "1.0",
  "format": "mtd",
  "partitions": [
    {
      "name": "bootinfo",
      "offset": "0",
      "size": "128K",
      "image": "factory/bootinfo_spinor.bin"
    },
    {
      "name": "fsbl",
      "offset": "128K",
      "size": "256K",
      "image": "factory/FSBL.bin"
    },
    {
      "name": "env",
      "offset": "384K",
      "size": "64K",
      "image": "env.bin"
    },
    {
      "name": "opensbi",
      "offset": "448K",
      "size": "192K",
      "image": "fw_dynamic.itb"
    },
    {
      "name": "uboot",
      "offset": "640K",
      "size": "-",
      "image": "u-boot.itb"
    }
  ]
}
"#;

const PARTITIONS_GPT_SPLIT: &str = r#"{
    "version": "1.0",
    "format": "gpt",
    "partitions": [
        {
            "name": "bootfs",
            "offset": "4M",
            "size": "256M",
            "image": "bootfs.img",
            "compress": "gzip-5"
        },
        {
            "name": "rootfs",
            "offset": "260M",
            "size": "-",
            "image": "rootfs.ext4",
            "compress": "gzip-5"
        }
    ]
}
"#;

const PARTITIONS_GPT_COMBINED: &str = r#"{
    "version": "1.0",
    "format": "gpt",
    "partitions": [
        {
            "name": "bootinfo",
            "hidden": true,
            "offset": "0K",
            "size": "512",
            "image": "factory/bootinfo_sd.bin",
            "holes": "{\"(80;512)\"}"
        },
        {
            "name": "fsbl",
            "offset": "128K",
            "size": "256K",
            "image": "factory/FSBL.bin"
        },
        {
            "name": "env",
            "offset": "384K",
            "size": "64K",
            "image": "env.bin"
        },
        {
            "name": "opensbi",
            "size": "384K",
            "image": "fw_dynamic.itb"
        },
        {
            "name": "uboot",
            "size": "2M",
            "image": "u-boot.itb"
        },
        {
            "name": "bootfs",
            "offset": "4M",
            "size": "256M",
            "image": "bootfs.img",
            "compress": "gzip-5"
        },
        {
            "name": "rootfs",
            "offset": "260M",
            "size": "-",
            "image": "rootfs.ext4",
            "compress": "gzip-5"
        }
    ]
}
"#;

fn is_spi_bootinfo(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.to_ascii_lowercase().contains("spi"))
}

pub(super) fn steps(artifacts: &[Artifact]) -> Result<Vec<Step>> {
    let bootinfo = Artifact::by_role(artifacts, Role::Bootinfo)?;

    let mut steps = vec![
        stage_step(artifacts, Role::Fsbl)?,
        stage_step(artifacts, Role::Uboot)?,
    ];

    if is_spi_bootinfo(&bootinfo.path) {
        steps.push(Step::FlashBytes {
            partition: "mtd",
            data: PARTITIONS_MTD.as_bytes(),
        });
        steps.push(Step::FlashBytes {
            partition: "gpt",
            data: PARTITIONS_GPT_SPLIT.as_bytes(),
        });
    } else {
        steps.push(Step::FlashBytes {
            partition: "gpt",
            data: PARTITIONS_GPT_COMBINED.as_bytes(),
        });
    }

    steps.extend([
        flash_step(artifacts, "bootinfo", Role::Bootinfo)?,
        flash_step(artifacts, "fsbl", Role::Fsbl)?,
        flash_step(artifacts, "env", Role::Env)?,
        flash_step(artifacts, "opensbi", Role::Opensbi)?,
        flash_step(artifacts, "uboot", Role::Uboot)?,
        flash_step(artifacts, "bootfs", Role::Boot)?,
        flash_step(artifacts, "rootfs", Role::Root)?,
    ]);

    Ok(steps)
}
