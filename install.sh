#!/usr/bin/env bash
# BlackShark V3 X (Xbox) Linux - complete installation
#   Rust daemon (blacksharkd) + CLI (blackshark) + Qt tray (blackshark-tray)
#
# Usage: ./install.sh
# sudo is requested for the udev rule; everything else is installed per user.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RS_DIR="$HERE/blackshark-rs"
QT_DIR="$HERE/blackshark-qt"
BIN_DIR="$HOME/.local/bin"

echo ">> Checking dependencies..."
command -v cargo >/dev/null || { echo "cargo is not installed. Install it from https://rustup.rs."; exit 1; }
command -v cmake >/dev/null || { echo "cmake is not installed. (sudo apt install cmake / pacman -S cmake)"; exit 1; }

mkdir -p "$BIN_DIR"

echo ">> Building Rust daemon and CLI..."
( cd "$RS_DIR" && cargo build --release )
install -m0755 "$RS_DIR/target/release/blacksharkd" "$BIN_DIR/blacksharkd"
install -m0755 "$RS_DIR/target/release/blackshark"  "$BIN_DIR/blackshark"

echo ">> Building Qt tray..."
cmake -S "$QT_DIR" -B "$QT_DIR/build" -DCMAKE_BUILD_TYPE=Release >/dev/null
cmake --build "$QT_DIR/build" -j"$(nproc)" >/dev/null
install -m0755 "$QT_DIR/build/blackshark-tray" "$BIN_DIR/blackshark-tray"

echo ">> Installing udev rule (sudo required)..."
sudo install -m0644 "$RS_DIR/99-blackshark.rules" /etc/udev/rules.d/99-blackshark.rules
sudo udevadm control --reload-rules
sudo udevadm trigger

echo ">> Installing systemd user service (daemon)..."
mkdir -p "$HOME/.config/systemd/user"
install -m0644 "$RS_DIR/systemd/blacksharkd.service" \
    "$HOME/.config/systemd/user/blacksharkd.service"
systemctl --user daemon-reload
systemctl --user enable --now blacksharkd.service

echo ">> Tray autostart..."
mkdir -p "$HOME/.config/autostart"
install -m0644 "$QT_DIR/pkg/blackshark-tray.desktop" \
    "$HOME/.config/autostart/blackshark-tray.desktop"

echo
echo "Installation complete."
echo "  Daemon  : systemctl --user status blacksharkd"
echo "  CLI     : blackshark status"
echo "  Tray    : blackshark-tray &   (starts automatically at the next login)"
echo
echo "Note: access is granted through the `users` group and uaccess."
