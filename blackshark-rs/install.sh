#!/usr/bin/env bash
# blackshark kurulum scripti (Linux, root gerekir).
#   sudo ./install.sh
set -euo pipefail

if [[ $EUID -ne 0 ]]; then
    echo "Bu script root gerektirir: sudo ./install.sh" >&2
    exit 1
fi

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo ">> Binary derleniyor (release)..."
# cargo root altinda calisirken sorun cikarmasin diye orijinal kullanici ile derle
BUILD_USER="${SUDO_USER:-root}"
sudo -u "$BUILD_USER" bash -c "cd '$REPO_DIR' && cargo build --release"

echo ">> Binary /usr/bin'e kopyalaniyor..."
install -m 0755 "$REPO_DIR/target/release/blackshark" /usr/bin/blackshark

echo ">> Config dizini olusturuluyor..."
mkdir -p /etc/blackshark
if [[ ! -f /etc/blackshark/config.toml ]]; then
    install -m 0644 "$REPO_DIR/config.example.toml" /etc/blackshark/config.toml
    echo "   Ornek config yazildi: /etc/blackshark/config.toml"
else
    echo "   Mevcut config korundu: /etc/blackshark/config.toml"
fi

echo ">> udev kurali kuruluyor..."
install -m 0644 "$REPO_DIR/99-blackshark.rules" /etc/udev/rules.d/99-blackshark.rules
udevadm control --reload-rules
udevadm trigger

echo ">> systemd servisleri kuruluyor..."
install -m 0644 "$REPO_DIR/systemd/blackshark.service" /etc/systemd/system/blackshark.service
install -m 0644 "$REPO_DIR/systemd/blackshark-apply@.service" /etc/systemd/system/blackshark-apply@.service
systemctl daemon-reload
systemctl enable blackshark.service

echo
echo "Kurulum tamam."
echo "  - Boot'ta ayarlar otomatik uygulanacak (blackshark.service)."
echo "  - Kulaklik takilinca da otomatik uygulanacak (udev -> blackshark-apply@)."
echo
echo "Deneme:"
echo "  blackshark battery"
echo "  blackshark eq-preset game     # cihaza uygular VE kaydeder"
echo "  blackshark apply              # kayitli ayarlari yeniden gonderir"
echo "  systemctl status blackshark.service"
