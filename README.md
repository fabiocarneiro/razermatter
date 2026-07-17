# RazerMatter

![GitHub Release](https://img.shields.io/github/v/release/fabiocarneiro/razermatter?style=for-the-badge)
![Rust](https://img.shields.io/badge/rust-stable-orange?style=for-the-badge&logo=rust)
![Matter](https://img.shields.io/badge/Matter-Certified-blue?style=for-the-badge)

RazerMatter is an Infrastructure Adapter and bridge that connects your Razer Chroma hardware natively into the modern **Matter** smart home ecosystem.

By translating standard Matter lighting concepts (Level Control, Color Control, On/Off) into raw Razer USB HID payloads, RazerMatter allows you to control your Razer devices—such as the Thunderbolt 4 Chroma Dock or Huntsman Keyboards—directly from apps like Google Home or Apple Home, without relying on any cloud services.

## Installation & Setup

### Automated Installation (Linux)

The easiest way to install RazerMatter is using the automated setup script. This script will download the latest pre-compiled binary, configure your USB permissions (`udev` rules), and set it up to run automatically in the background as a `systemd` service.

Just run this single command in your terminal:

```bash
curl -sSL https://raw.githubusercontent.com/fabiocarneiro/razermatter/master/install.sh | bash
```

Once the script finishes, your daemon will be fully installed and running.

To pair your bridge to Google Home or Apple Home, simply type this command in your terminal:
```bash
razermatter-pair
```
This utility will safely extract the QR code and print it beautifully into your terminal for you to scan.

> [!TIP]
> **Resetting the Pairing State**
> If you ever need to factory-reset the bridge to pair it with a different home network, you can run:
> ```bash
> razermatter-pair --reset
> ```
> This will automatically stop the service, clear the secure pairing storage (`/tmp/rs-matter`), restart the bridge, and print out a brand new QR Code for you to scan.

<details>
<summary><b>Manual Installation & Compilation (For Developers)</b></summary>

If you prefer to compile from source, make sure you have the latest stable Rust toolchain and `hidapi` dependencies (e.g. `libudev-dev`, `pkg-config` on Linux).

To build the daemon in release mode:

```bash
cargo build --release
```
The resulting binary will be located at `./target/release/razermatter`.

**Configure USB Permissions (Linux udev rules)**

1. Create a file at `/etc/udev/rules.d/99-razer.rules`:
```bash
# Allow users in the "plugdev" group to access the Razer Thunderbolt 4 Dock
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="1532", ATTRS{idProduct}=="0f21", MODE="0660", GROUP="plugdev"
# Allow users in the "plugdev" group to access the Razer Huntsman Tournament Edition
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="1532", ATTRS{idProduct}=="0243", MODE="0660", GROUP="plugdev"
```

2. Reload the udev rules and re-plug your devices:
```bash
sudo udevadm control --reload-rules
sudo udevadm trigger
```

**Run the daemon**

Once compiled and authorized via `udev`, you can run the daemon directly:

```bash
./target/release/razermatter
```
</details>

## Features

- **Matter Certified Emulation:** Exposes your hardware as a standard Matter `EXTENDED_COLOR_LIGHT`.
- **Local Control Only:** No cloud servers, no account logins, just pure local IPv6 control.
- **Full RGB Support:** Change colors smoothly using the color wheel in your smart home app.
- **Brightness Dimming:** Supports 1-254 hardware-level dimming.
- **Reverse-Engineered HID:** Communicates directly with the Razer hardware via `hidapi` using precise 90-byte USB payload sequences. It correctly targets the specific HID interfaces for lighting commands (`usage_page 0x000C` for the Dock, and `interface 2` for the Keyboard) to prevent interference with standard inputs.

## Current Device Support

- Razer Thunderbolt 4 Dock Chroma (VID: `0x1532`, PID: `0x0F21`)
- Razer Huntsman Tournament Edition (VID: `0x1532`, PID: `0x0243`)
*(Note: Support for other Razer devices can be added by duplicating the endpoints and updating USB PID targets).*

## Architecture

The project is logically structured with modularity in mind:
- `src/hardware`: Handles raw USB HID device communication. This layer is decoupled and vendor-agnostic, accepting raw byte payloads.
- `src/protocol`: Contains proprietary byte-level payload assembly and CRC calculation for Razer devices, purely mathematically and completely separate from hardware IO.
- `src/bridge`: Handles the Matter server, MDNS orchestration, cluster state logic (OnOff, LevelControl, ColorControl), and mapping to the hardware layer. 
- `src/main.rs`: A minimal entry point that boots the Matter server.

## Privacy & Security Considerations

### No Hardcoded Secrets
This project contains no hardcoded personal network information (no SSIDs, IPs, or MAC addresses). The Matter device utilizes public CHIP testing certificates during development. All pairing states (such as your fabric IDs) are stored securely in your system's temp directory (`/tmp/rs-matter`) and are explicitly git-ignored.
