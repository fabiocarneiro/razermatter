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

# 6. Create pair utility
echo "[5/6] Installing 'razermatter-pair' utility..."
sudo bash -c 'cat > /usr/local/bin/razermatter-pair <<EOF
#!/usr/bin/env bash

if [ "\$1" == "--reset" ]; then
    echo "Resetting RazerMatter pairing state..."
    sudo systemctl stop razermatter.service
    TARGET_USER="\${SUDO_USER:-\$USER}"
    sudo rm -rf "/home/\$TARGET_USER/.razermatter"
    sudo systemctl start razermatter.service
fi

sudo /usr/local/bin/razermatter --qr-only
EOF'
sudo chmod +x /usr/local/bin/razermatter-pair

# Remove old reset utility if it exists
sudo rm -f /usr/local/bin/razermatter-reset

# 7. Setup systemd service
echo "[6/7] Configuring background service (systemd)..."
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

# 8. Enable and start
echo "[7/7] Starting RazerMatter service..."
sudo systemctl daemon-reload
sudo systemctl enable razermatter.service
sudo systemctl restart razermatter.service

echo "================================================="
echo "Installation Complete! RazerMatter is running in the background."
echo "================================================="
echo ""

# Automatically run the pairing utility to show the QR code (if not already paired)
/usr/local/bin/razermatter-pair
