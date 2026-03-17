#!/usr/bin/env python3
import subprocess
from datetime import datetime


# Get's information about the USB device from udevadm
def get_device_info(device_path):
    # To ensure we get e.g. 1-4, instead of the full device_path
    device_name = device_path.split("/")[-1]
    result = subprocess.run(
            ["udevadm", "info", f"/sys/bus/usb/devices/{device_name}"],
            capture_output=True,
            text=True
    )

    props = {}
    for line in result.stdout.splitlines():
        if line.startswith("E: "):
            key, value = line[3:].split("=", 1)
            props[key] = value

    return props

# Extracts information about the device
def extract_info(props):
    return {
            "vendor": props.get("ID_VENDOR_FROM_DATABASE", "Unknown vendor"),
            "model": props.get("ID_MODEL_FROM_DATABASE", "Unknown model"),
            "serial": props.get("ID_SERIAL_SHORT", "Unknown serial number"),
            "vid": props.get("ID_USB_VENDOR_ID", "?"),
            "pid": props.get("ID_USB_MODEL_ID", "?"),
    }

# Formats an event to readable text
def format_event(action, info):
    time = datetime.now().strftime("%H:%M:%S")
    return (
            f"[{time}] {action.upper()}: {info['vendor']} - {info['model']}\n"
            f" ID: {info['vid']}:{info['pid']} | Serial number: {info['serial']}"
    )

# Listens on USB events
def listener():
    # Cache added devices info is available when REMOVED
    known_devices = {}

    proc = subprocess.Popen(
            ["udevadm", "monitor", "--subsystem-match=usb", "--udev"],
            stdout=subprocess.PIPE,
            text=True,
    )

    for line in proc.stdout:
        parts = line.strip().split()
        if len(parts) >= 4 and parts[0] == "UDEV":
            action = parts[2]
            device_path = parts[3]

            # Only for the main device, not per interface
            if ":" not in device_path.split("/")[-1]:
                if action == "add":
                    props = get_device_info(device_path)
                    known_devices[device_path] = props
                elif action == "remove":
                    props = known_devices.pop(device_path, {})
                else:
                    continue

                info = extract_info(props)
                print(format_event(action, info))
                notify(action, info)
                print()


# Sends notification with notify-send
def notify(action, info):
    if action == "add": 
        title = "USB Connected"
    else:
        title = "USB Disconnected"

    icon = "/usr/share/icons/breeze-dark/preferences/32/device-notifier.svg"

    body = f"{info['vendor']} - {info['model']}\nSerial nr: {info['serial']}"

    subprocess.run([
        "notify-send",
        "--icon", icon,
        title,
        body,
    ])

if __name__ == "__main__":
    listener()
