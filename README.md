# RazerMatter

RazerMatter is an Anti-Corruption Layer (ACL) and bridge that connects your Razer Chroma hardware natively into the modern **Matter** smart home ecosystem.

By translating standard Matter lighting concepts (Level Control, Color Control, On/Off) into raw Razer USB HID payloads, RazerMatter allows you to control your Razer devices—such as the Thunderbolt 4 Chroma Dock—directly from apps like Google Home or Apple Home, without relying on any cloud services.

## Features

- **Matter Certified Emulation:** Exposes your hardware as a standard Matter `EXTENDED_COLOR_LIGHT`.
- **Local Control Only:** No cloud servers, no account logins, just pure local IPv6 control.
- **Full RGB Support:** Change colors smoothly using the color wheel in your smart home app.
- **Brightness Dimming:** Supports 1-254 hardware-level dimming.
- **Reverse-Engineered HID:** Communicates directly with the Razer hardware via `hidapi` using precise 90-byte USB payload sequences (and specifically the `0x1F` transaction ID for the Thunderbolt 4 Dock).

## Current Device Support

- Razer Thunderbolt 4 Dock Chroma (VID: `0x1532`, PID: `0x0F21`)
*(Note: Support for other Razer devices can be added by duplicating the endpoints and updating USB PID targets).*

## Prerequisites

- **Rust:** You'll need the latest stable Rust toolchain to build this project.
- **`hidapi` dependencies:** Make sure you have the required native libraries installed (e.g. `libhidapi-hidraw0`, `libusb-1.0-0-dev` on Linux).
- A Matter controller (like a Google Nest Hub or Apple HomePod).

## Building & Running

To build the daemon in release mode:

```bash
cargo build --release
```

Since the application requires raw USB access to the Razer dock, you must run it with elevated privileges:

```bash
sudo ./target/release/razermatter
```

When you first launch the daemon, it will print a standard Matter Pairing Code and a QR code in the terminal. You can scan this QR code using the Google Home or Apple Home app to pair the dock to your network.

## Testing Your Dock

If you'd like to test the raw USB HID commands without launching the full Matter stack, a small test binary is included:

```bash
cargo build --bin test_dock
sudo ./target/debug/test_dock
```

This will automatically cycle the dock through brightness off/on and static colors to ensure your USB payload logic is working.

## Privacy & Security

This project contains no hardcoded personal network information (no SSIDs, IPs, or MAC addresses). The Matter device utilizes public CHIP testing certificates during development. All pairing states (such as your fabric IDs) are stored securely in your system's temp directory (`/tmp/rs-matter`) and are explicitly git-ignored.
