//! USB Fastboot device discovery via `nusb`.

use nusb::{DeviceInfo, MaybeFuture};

use super::{Enumerator, Target, usb_target};
pub use super::{matches, same_port};

const FASTBOOT_CLASS: u8 = 0xff;
const FASTBOOT_SUBCLASS: u8 = 0x42;
const FASTBOOT_PROTOCOL: u8 = 0x03;

pub(crate) fn scan() -> Vec<Target> {
    let Ok(devices) = nusb::list_devices().wait() else {
        return Vec::new();
    };

    devices
        .filter(is_fastboot_device)
        .map(|info| usb_target(info, Enumerator::UsbFastboot, "USB Fastboot"))
        .collect()
}

pub(super) fn is_fastboot_device(info: &DeviceInfo) -> bool {
    info.interfaces().any(|iface| {
        iface.class() == FASTBOOT_CLASS
            && iface.subclass() == FASTBOOT_SUBCLASS
            && iface.protocol() == FASTBOOT_PROTOCOL
    })
}
