#!/usr/bin/env bash
set -e

echo "================================================="
echo "       RazerMatter Auto-Install Script"
echo "================================================="

# 1. System checks
if [ "$(uname)" != "Linux" ]; then
    echo "Error: This script is only intended for Linux."
    exit 1
fi

if [ "$(uname -m)" != "x86_64" ]; then
    echo "Error: This script currently only supports x86_64 architecture."
    exit 1
fi

if ! command -v curl &> /dev/null; then
    echo "Error: curl is required but not installed. Please install it."
    exit 1
fi

USER_NAME="${SUDO_USER:-$USER}"
if [ "$USER_NAME" = "root" ]; then
    echo "Error: Please run this script as your normal user, not as root directly."
    exit 1
fi

# 2. Fetch the latest release URL
echo "[1/6] Finding the latest release..."
LATEST_TAG=$(curl -sSL "https://api.github.com/repos/fabiocarneiro/razermatter/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

if [ -z "$LATEST_TAG" ]; then
    echo "Error: Could not determine the latest release from GitHub."
    exit 1
fi

DOWNLOAD_URL="https://github.com/fabiocarneiro/razermatter/releases/download/${LATEST_TAG}/razermatter-linux-x86_64"

# 3. Stop service if it exists
if systemctl is-active --quiet razermatter.service; then
    echo "[2/6] Stopping existing service..."
    sudo systemctl stop razermatter.service
else
    echo "[2/6] Preparing installation..."
fi

# 4. Download the binary
echo "[3/6] Downloading RazerMatter ${LATEST_TAG}..."
curl -sSL -o /tmp/razermatter_download "$DOWNLOAD_URL"
chmod +x /tmp/razermatter_download
sudo mv /tmp/razermatter_download /usr/local/bin/razermatter

# 5. Setup udev rules
echo "[4/6] Configuring USB permissions (udev)..."
sudo bash -c 'cat > /etc/udev/rules.d/99-razer.rules <<EOF
# Allow users in the "plugdev" group to access the Razer Thunderbolt 4 Dock
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="1532", ATTRS{idProduct}=="0f21", MODE="0660", GROUP="plugdev"
# Allow users in the "plugdev" group to access the Razer Huntsman Tournament Edition
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="1532", ATTRS{idProduct}=="0243", MODE="0660", GROUP="plugdev"
EOF'

# Ensure the user is in the plugdev group
if ! getent group plugdev >/dev/null; then
    sudo groupadd plugdev
fi
sudo usermod -aG plugdev "$USER_NAME"

sudo udevadm control --reload-rules
sudo udevadm trigger

# 6. Setup systemd service
echo "[5/6] Configuring background service (systemd)..."
sudo bash -c "cat > /etc/systemd/system/razermatter.service <<EOF
[Unit]
Description=RazerMatter Smart Home Bridge
After=network.target

[Service]
User=$USER_NAME
Group=plugdev
WorkingDirectory=/home/$USER_NAME
ExecStart=/usr/local/bin/razermatter
Restart=always
RestartSec=3

[Install]
WantedBy=multi-user.target
EOF"

# 7. Enable and start
echo "[6/6] Starting RazerMatter service..."
sudo systemctl daemon-reload
sudo systemctl enable razermatter.service
sudo systemctl restart razermatter.service

echo "Waiting for the Matter pairing code to generate..."
sleep 4

echo "================================================="
echo "            Matter Pairing QR Code"
echo "================================================="
echo ""
# Extract the QR code from the logs, strip away the timestamp/log prefix, and take only the last 19 lines (1 full code)
journalctl -u razermatter.service -n 100 --no-pager | grep "██" | sed -E 's/.*INFO.*rs_matter\] //' | tail -n 19
echo ""
echo "================================================="
echo "Installation Complete! RazerMatter is running in the background."
echo "Scan the QR code above using the Google Home or Apple Home app."
echo ""
echo "If the QR code is cut off or your terminal is too narrow, you can also"
echo "view the manual 11-digit text pairing code by checking the logs:"
echo "    journalctl -u razermatter.service -n 50 --no-pager"
echo "================================================="
