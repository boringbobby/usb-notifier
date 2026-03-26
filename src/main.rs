use notify_rust::Notification;
use std::collections::HashMap;
use std::os::unix::io::AsRawFd;

/// Struct for information about a USB device.
struct DeviceInfo {
    vendor: String,
    model: String,
}

/// Extract device info from a udev device, walking up to the parent if needed.
fn extract_info(device: &udev::Device) -> DeviceInfo {
    DeviceInfo {
        vendor: get_property_with_fallback(device, &[
            Property::UdevProperty("ID_VENDOR_FROM_DATABASE"),
            Property::SysAttr("manufacturer"),
            Property::UdevProperty("ID_VENDOR"),
        ])
        .unwrap_or_else(|| "Unknown vendor".into()),

        model: get_property_with_fallback(device, &[
            Property::UdevProperty("ID_MODEL_FROM_DATABASE"),
            Property::SysAttr("product"),
            Property::UdevProperty("ID_MODEL"),
        ])
        .unwrap_or_else(|| "Unknown model".into()),
    }
}

/// The two places we can look up a value on a udev device.
enum Property<'a> {
    UdevProperty(&'a str),
    SysAttr(&'a str),
}

/// Try each source in order, return the first non-empty value.
fn get_property_with_fallback(device: &udev::Device, sources: &[Property]) -> Option<String> {
    for source in sources {
        let value = match source {
            Property::UdevProperty(key) => device.property_value(key),
            Property::SysAttr(key) => device.attribute_value(key),
        };

        if let Some(v) = value {
            let s = v.to_string_lossy();
            if !s.is_empty() {
                return Some(s.into_owned());
            }
        }
    }
    None
}


/// Check if a device is a top-level USB device (not an interface).
fn is_usb_device(device: &udev::Device) -> bool {
    device
        .property_value("DEVTYPE")
        .is_some_and(|v| v == "usb_device")
}


/// Send a desktop notification.
fn send_notification(action: &str, info: &DeviceInfo) {
    let title = match action {
        "add" => "USB Connected",
        "remove" => "USB Disconnected",
        _ => "USB Event",
    };

    let body = format!(
        "{} - {}",
        info.vendor, info.model,
    );

    let _  = Notification::new() 
        .summary(title)
        .body(&body)
        .icon("drive-removable-media")
        .show();
}

/// Scan all currently connected USB devices.
fn scan_existing_devices(known_devices: &mut HashMap<String, DeviceInfo>) {
    let mut enumerator = udev::Enumerator::new().expect("Failed to create udev enumerator");
    enumerator.match_subsystem("usb").unwrap();
    enumerator.match_property("DEVTYPE", "usb_device").unwrap();

    let devices: Vec<_> = enumerator
        .scan_devices()
        .expect("Failed to scan devices")
        .collect();

    for device in devices {
        let info = extract_info(&device);
        let syspath = device.syspath().to_string_lossy().into_owned();

        known_devices.insert(syspath, info);
    }
}

/// Listen for USB add/remove events and notify.
fn listen(known_devices: &mut HashMap<String, DeviceInfo>) {
    let socket = udev::MonitorBuilder::new()
        .expect("Failed to create udev monitor")
        .match_subsystem("usb")
        .expect("Failed to match subsystem")
        .listen()
        .expect("Failed to start listening");

    let fd = socket.as_raw_fd();

    loop {
        // Block until the socket has data
        let mut fds = [libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        }];
        if unsafe { libc::poll(fds.as_mut_ptr(), 1, -1) } < 0 {
            eprintln!("poll() failed");
            break;
        }

    for event in socket.iter() {
        let action = match event.action() {
            Some(a) => a,
            None => continue,
        };

        if !is_usb_device(&event) {
            continue;
        }

        let syspath = event.syspath().to_string_lossy().into_owned();
        let action_str = action.to_string_lossy();

        match action_str.as_ref() {
            "add" => {
                let info = extract_info(&event);
                send_notification("add", &info);
                known_devices.insert(syspath, info);
            }
            "remove" => {
                // Use cached info if available (device properties are gone on removal)
                let info = known_devices
                    .remove(&syspath)
                    .unwrap_or_else(|| extract_info(&event));

                send_notification("remove", &info);
            }
            _ => {}
        }
    }
    } // loop
}

fn main() {
    let mut known_devices: HashMap<String, DeviceInfo> = HashMap::new();

    scan_existing_devices(&mut known_devices);
    listen(&mut known_devices);
}
