//! USB Fastboot device discovery via `nusb`.

use std::path::{Path, PathBuf};

use nusb::{DeviceInfo, MaybeFuture};

use crate::targets::{BusType, Enumerator, Target};

const FASTBOOT_CLASS: u8 = 0xff;
const FASTBOOT_SUBCLASS: u8 = 0x42;
const FASTBOOT_PROTOCOL: u8 = 0x03;

pub(crate) fn scan() -> Vec<Target> {
    let Ok(devices) = nusb::list_devices().wait() else {
        return Vec::new();
    };

    devices
        .filter(is_fastboot_device)
        .map(into_target)
        .collect()
}

fn is_fastboot_device(info: &DeviceInfo) -> bool {
    info.interfaces().any(|iface| {
        iface.class() == FASTBOOT_CLASS
            && iface.subclass() == FASTBOOT_SUBCLASS
            && iface.protocol() == FASTBOOT_PROTOCOL
    })
}

fn into_target(info: DeviceInfo) -> Target {
    let (device, device_path) = target_paths(&info);
    let description = format_description(
        info.manufacturer_string(),
        info.product_string(),
        info.serial_number(),
        info.vendor_id(),
        info.product_id(),
    );

    Target {
        enumerator: Enumerator::UsbFastboot,
        bus_type: BusType::Usb,
        bus_version: format_usb_version(info.usb_version()),
        raw: device.clone(),
        device,
        device_path,
        description,
        size: None,
        block_size: 0,
        logical_block_size: 0,
        mountpoints: Vec::new(),
        is_read_only: false,
        is_system: false,
        is_virtual: false,
        is_removable: true,
        is_card: None,
        is_uas: None,
        partition_table: None,
    }
}

pub fn matches(info: &DeviceInfo, target: &Path) -> bool {
    let (device, device_path) = target_paths(info);
    target == device || device_path.as_deref() == Some(target)
}

pub fn same_port(left: &DeviceInfo, right: &DeviceInfo) -> bool {
    left.bus_id() == right.bus_id() && left.port_chain() == right.port_chain()
}

#[cfg(target_os = "linux")]
fn target_paths(info: &DeviceInfo) -> (PathBuf, Option<PathBuf>) {
    let device = PathBuf::from(format!(
        "/dev/bus/usb/{:03}/{:03}",
        info.busnum(),
        info.device_address()
    ));
    let device_path = Some(info.sysfs_path().to_path_buf());
    (device, device_path)
}

#[cfg(not(target_os = "linux"))]
fn target_paths(info: &DeviceInfo) -> (PathBuf, Option<PathBuf>) {
    (PathBuf::from(usb_port_id(info)), None)
}

#[cfg(not(target_os = "linux"))]
fn usb_port_id(info: &DeviceInfo) -> String {
    if info.port_chain().is_empty() {
        format!("usb:{}:{}", info.bus_id(), info.device_address())
    } else {
        let ports = info
            .port_chain()
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(".");
        format!("usb:{}-{ports}", info.bus_id())
    }
}

fn format_usb_version(bcd: u16) -> Option<String> {
    if bcd == 0 {
        return None;
    }

    let major = ((bcd >> 12) & 0xf) * 10 + ((bcd >> 8) & 0xf);
    let minor_tens = (bcd >> 4) & 0xf;
    let minor_ones = bcd & 0xf;

    if minor_ones == 0 {
        Some(format!("{major}.{minor_tens}"))
    } else {
        Some(format!("{major}.{minor_tens}{minor_ones}"))
    }
}

fn format_description(
    manufacturer: Option<&str>,
    product: Option<&str>,
    serial: Option<&str>,
    vendor_id: u16,
    product_id: u16,
) -> String {
    let words: Vec<&str> = [manufacturer, product, serial]
        .into_iter()
        .flatten()
        .flat_map(str::split_whitespace)
        .collect();

    let label = if words.is_empty() {
        "USB Fastboot".to_owned()
    } else {
        words.join(" ")
    };

    format!("{label} ({vendor_id:04x}:{product_id:04x})")
}
