//! USB RockUSB device discovery via `nusb`.

pub use nusb::DeviceInfo;
use nusb::MaybeFuture;

use super::{Enumerator, Target, usb_target};
pub use super::{matches, same_port};

const ROCKCHIP_VENDOR_ID: u16 = 0x2207;

pub(crate) fn scan() -> Vec<Target> {
    let Ok(devices) = nusb::list_devices().wait() else {
        return Vec::new();
    };

    devices
        .filter(is_rockusb_device)
        .map(|info| usb_target(info, Enumerator::UsbRockusb, "USB Rockusb"))
        .collect()
}

fn is_rockusb_device(info: &DeviceInfo) -> bool {
    info.vendor_id() == ROCKCHIP_VENDOR_ID && !super::fastboot::is_fastboot_device(info)
}
