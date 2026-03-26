# usb-notifier

A lightweight USB device monitor for Linux that sends desktop notifications when USB devices are connected or disconnected.

## Build

Requires Rust and 'pkg-config' / 'pkgconf'.

'''bash
cargo build --release
'''

"" Autostart
*Note*: First, add usb-notifier to PATH.

Hyprland:
'''
exec-once = usb-notifier
'''

Sway:
'''
exec usb-notifier
'''

## License

MIT
