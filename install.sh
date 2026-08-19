#!/usr/bin/env bash
# BlackShark V3 X (Xbox) Linux - tam kurulum
#   Rust daemon (blacksharkd) + CLI (blackshark) + Qt tray (blackshark-tray)
#
# Kullanim:  ./install.sh
# udev kurali icin sudo sorulacak; gerisi kullanici seviyesinde.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RS_DIR="$HERE/blackshark-rs"
QT_DIR="$HERE/blackshark-qt"
BIN_DIR="$HOME/.local/bin"

echo ">> Bagimlilik kontrolu..."
command -v cargo >/dev/null || { echo "cargo yok. https://rustup.rs kur."; exit 1; }
command -v cmake >/dev/null || { echo "cmake yok. (sudo apt install cmake / pacman -S cmake)"; exit 1; }

mkdir -p "$BIN_DIR"

echo ">> Rust daemon + CLI derleniyor..."
( cd "$RS_DIR" && cargo build --release )
install -m0755 "$RS_DIR/target/release/blacksharkd" "$BIN_DIR/blacksharkd"
install -m0755 "$RS_DIR/target/release/blackshark"  "$BIN_DIR/blackshark"

echo ">> Qt tray derleniyor..."
cmake -S "$QT_DIR" -B "$QT_DIR/build" -DCMAKE_BUILD_TYPE=Release >/dev/null
cmake --build "$QT_DIR/build" -j"$(nproc)" >/dev/null
install -m0755 "$QT_DIR/build/blackshark-tray" "$BIN_DIR/blackshark-tray"

echo ">> udev kurali (sudo gerekir)..."
sudo install -m0644 "$RS_DIR/99-blackshark.rules" /etc/udev/rules.d/99-blackshark.rules
sudo udevadm control --reload-rules
sudo udevadm trigger

echo ">> systemd user servisi (daemon)..."
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
echo "Kurulum tamam."
echo "  Daemon  : systemctl --user status blacksharkd"
echo "  CLI     : blackshark status"
echo "  Tray    : blackshark-tray &   (bir sonraki oturumda otomatik acilir)"
echo
echo "Not: 'plugdev' grubunda degilsen:  sudo usermod -aG plugdev \$USER  (sonra yeniden giris)"
