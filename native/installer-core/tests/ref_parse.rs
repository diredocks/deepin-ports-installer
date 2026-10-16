use std::path::PathBuf;

use installer_core::{DeviceIndex, ImageIndex};

fn ref_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../ref")
        .join(name)
}

#[test]
fn parses_real_ref_files() {
    let devices = DeviceIndex::parse_file(ref_path("devices.toml")).unwrap();
    assert_eq!(devices.len(), 12);
    assert_eq!(devices.groups()[0].soc, "jh7110");

    let images = ImageIndex::parse_file(ref_path("images.toml")).unwrap();
    assert!(!images.is_empty());
    let first = &images.groups()[0];
    assert_eq!(
        (first.arch.as_str(), first.device.as_str()),
        ("riscv", "dp1000")
    );
}
