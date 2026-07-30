#!/usr/bin/env bash
set -e

echo "================================================="
echo "       RazerMatter Uninstall Script"
echo "================================================="

if [ "$(uname)" != "Linux" ]; then
    echo "Error: This script is only intended for Linux."
    exit 1
fi

echo "[1/4] Stopping and removing systemd service..."
if systemctl is-active --quiet razermatter.service || systemctl is-enabled --quiet razermatter.service 2>/dev/null; then
    sudo systemctl stop razermatter.service || true
    sudo systemctl disable razermatter.service || true
fi
sudo rm -f /etc/systemd/system/razermatter.service
sudo systemctl daemon-reload

echo "[2/4] Removing binaries..."
sudo rm -f /usr/local/bin/razermatter
sudo rm -f /usr/local/bin/razermatter-pair
sudo rm -f /usr/local/bin/razermatter-reset

echo "[3/4] Removing udev rules..."
sudo rm -f /etc/udev/rules.d/99-razer.rules
sudo udevadm control --reload-rules
sudo udevadm trigger

echo "[4/4] Restoring power management settings..."
read -p "Would you like to restore sleep and hibernation if they were disabled? [Y/n]: " -r RESTORE_SLEEP < /dev/tty || RESTORE_SLEEP="y"
if [[ ! "$RESTORE_SLEEP" =~ ^[Nn]$ ]]; then
    echo "Unmasking sleep and hibernation targets..."
    sudo systemctl unmask sleep.target suspend.target hibernate.target hybrid-sleep.target suspend-then-hibernate.target
else
    echo "Leaving sleep settings unchanged."
fi

echo "================================================="
echo "Uninstallation Complete!"
echo "Note: Any Matter pairing data files left in your home directory will need to be removed manually."
echo "================================================="
