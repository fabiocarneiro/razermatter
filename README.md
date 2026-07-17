# RazerMatter

RazerMatter is an Infrastructure Adapter and bridge that connects your Razer Chroma hardware natively into the modern **Matter** smart home ecosystem.

By translating standard Matter lighting concepts (Level Control, Color Control, On/Off) into raw Razer USB HID payloads, RazerMatter allows you to control your Razer devices—such as the Thunderbolt 4 Chroma Dock or Huntsman Keyboards—directly from apps like Google Home or Apple Home, without relying on any cloud services.

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

The project is structured with strict Separation of Concerns (SRP) in mind:
- `src/hardware`: Handles raw USB HID device communication. This layer is decoupled and vendor-agnostic, accepting raw byte payloads.
- `src/protocol`: Contains proprietary byte-level payload assembly and CRC calculation for Razer devices, purely mathematically and completely separate from hardware IO.
- `src/bridge`: Handles the Matter server, MDNS orchestration, cluster state logic (OnOff, LevelControl, ColorControl), and mapping to the hardware layer. 
- `src/main.rs`: A minimal entry point that boots the Matter server.

## Prerequisites

- **Rust:** You'll need the latest stable Rust toolchain to build this project.
- **`hidapi` dependencies:** Make sure you have the required native libraries installed (e.g. `libhidapi-hidraw0`, `libusb-1.0-0-dev` on Linux).
- A Matter controller (like a Google Nest Hub or Apple HomePod).

## Installation & Setup

### 1. Download the Binary

The easiest way to install RazerMatter is to download the pre-compiled binary for your operating system from the [GitHub Releases page](https://github.com/fabiocarneiro/razermatter/releases).

1. Go to the **Releases** page.
2. Download the appropriate binary for your system (e.g., `razermatter-linux-x86_64`).
3. Make it executable:
```bash
chmod +x razermatter-linux-x86_64
```

<details>
<summary><b>Alternative: Building from Source</b></summary>

If you prefer to compile from source, make sure you have the latest stable Rust toolchain and `hidapi` dependencies (e.g. `libudev-dev`, `pkg-config` on Linux).

To build the daemon in release mode:

```bash
cargo build --release
```
The resulting binary will be located at `./target/release/razermatter`.
</details>

### 2. Configure USB Permissions (Linux udev rules)

By default, the Linux kernel restricts raw USB HID access to the `root` user. For better system security, it is highly recommended to run this daemon as a standard user instead of using `sudo`.

You can grant your user permission to access the Razer devices by creating a `udev` rule:

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
*(Make sure your user is part of the `plugdev` group using `sudo usermod -aG plugdev $USER`)*.

### 3. Run the daemon

Once downloaded (or compiled) and authorized via `udev`, you can run the daemon directly:

```bash
./razermatter-linux-x86_64
```
*(If you skipped the udev rules, you will need to run this with `sudo`)*.

When you first launch the daemon, it will print a standard Matter Pairing Code and a QR code in the terminal. You can scan this QR code using the Google Home or Apple Home app to pair the bridge to your network. Once paired, all supported devices will appear as separate lights!

### 4. Running as a Service (systemd on Linux)

To ensure the bridge starts automatically whenever your computer boots up, you can set it up as a background `systemd` service.

1. Move the binary to a permanent location, such as `/usr/local/bin/`:
```bash
sudo mv razermatter-linux-x86_64 /usr/local/bin/razermatter
```

2. Create a service file at `/etc/systemd/system/razermatter.service` (you'll need `sudo`):
```ini
[Unit]
Description=RazerMatter Smart Home Bridge
After=network.target

[Service]
# If you configured the udev rules above, you can run this as your standard user!
# Otherwise, change this to User=root
User=<your-username>
Group=plugdev
WorkingDirectory=/home/<your-username>
ExecStart=/usr/local/bin/razermatter
Restart=always
RestartSec=3

[Install]
WantedBy=multi-user.target
```
*(Note: Replace `<your-username>` with your actual Linux username).*

3. Enable and start the service:
```bash
sudo systemctl daemon-reload
sudo systemctl enable razermatter.service
sudo systemctl start razermatter.service
```

You can check its logs at any time using `journalctl -u razermatter.service -f`.



## Privacy & Security Considerations

### No Hardcoded Secrets
This project contains no hardcoded personal network information (no SSIDs, IPs, or MAC addresses). The Matter device utilizes public CHIP testing certificates during development. All pairing states (such as your fabric IDs) are stored securely in your system's temp directory (`/tmp/rs-matter`) and are explicitly git-ignored.
