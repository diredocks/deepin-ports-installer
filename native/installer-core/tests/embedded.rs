use installer_core::{DeviceIndex, ImageIndex};

#[test]
fn parses_embedded_indexes() {
    let devices = DeviceIndex::embedded().unwrap();
    assert_eq!(devices.len(), 12);
    assert_eq!(devices.groups()[0].soc, "jh7110");

    let images = ImageIndex::embedded().unwrap();
    assert!(!images.is_empty());
    let first = &images.groups()[0];
    assert_eq!(
        (first.arch.as_str(), first.device.as_str()),
        ("riscv", "dp1000")
    );
}
