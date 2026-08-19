#!/usr/bin/env bash
# blackshark installation script (Linux; requires root).
#   sudo ./install.sh
set -euo pipefail

if [[ $EUID -ne 0 ]]; then
    echo "This script requires root: sudo ./install.sh" >&2
    exit 1
fi

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo ">> Building binary (release)..."
# Build as the original user to avoid problems when Cargo runs as root.
BUILD_USER="${SUDO_USER:-root}"
sudo -u "$BUILD_USER" bash -c "cd '$REPO_DIR' && cargo build --release"

echo ">> Copying binary to /usr/bin..."
install -m 0755 "$REPO_DIR/target/release/blackshark" /usr/bin/blackshark

echo ">> Creating configuration directory..."
mkdir -p /etc/blackshark
if [[ ! -f /etc/blackshark/config.toml ]]; then
    install -m 0644 "$REPO_DIR/config.example.toml" /etc/blackshark/config.toml
    echo "   Example configuration written: /etc/blackshark/config.toml"
else
    echo "   Existing configuration preserved: /etc/blackshark/config.toml"
fi

echo ">> Installing udev rule..."
install -m 0644 "$REPO_DIR/99-blackshark.rules" /etc/udev/rules.d/99-blackshark.rules
udevadm control --reload-rules
udevadm trigger

echo ">> Installing systemd services..."
install -m 0644 "$REPO_DIR/systemd/blackshark.service" /etc/systemd/system/blackshark.service
install -m 0644 "$REPO_DIR/systemd/blackshark-apply@.service" /etc/systemd/system/blackshark-apply@.service
systemctl daemon-reload
systemctl enable blackshark.service

echo
echo "Installation complete."
echo "  - Settings will be applied automatically at boot (blackshark.service)."
echo "  - Settings will also be applied when the headset is connected (udev -> blackshark-apply@)."
echo
echo "Try:"
echo "  blackshark battery"
echo "  blackshark eq-preset game     # applies to the device and saves it"
echo "  blackshark apply              # re-sends the saved settings"
echo "  systemctl status blackshark.service"
