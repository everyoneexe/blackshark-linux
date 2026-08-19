#!/usr/bin/env bash
# Probe the BlackShark BLE control endpoint without leaving the audio device blocked.
#
# The headset exposes classic Bluetooth for audio and a separate BLE endpoint for
# Razer Audio Companion controls. The endpoint currently observed on Android is
# FB:5E:CD:E8:F5:F4; the classic audio address is 44:5E:CD:E8:F5:F4.
set -euo pipefail

CLASSIC_ADDRESS="44:5E:CD:E8:F5:F4"
BLE_ADDRESS="FB:5E:CD:E8:F5:F4"
SCAN_SECONDS="${1:-30}"
HCI_INDEX="${BLACKSHARK_HCI_INDEX:-0}"

if ! [[ "$SCAN_SECONDS" =~ ^[1-9][0-9]*$ ]]; then
    echo "Usage: $0 [scan-seconds]" >&2
    exit 2
fi

command -v bluetoothctl >/dev/null || {
    echo "bluetoothctl is required (BlueZ)." >&2
    exit 1
}
command -v btmgmt >/dev/null || {
    echo "btmgmt is required (it is provided by BlueZ)." >&2
    exit 1
}

was_connected=false
was_bredr_enabled=false
if bluetoothctl info "$CLASSIC_ADDRESS" 2>/dev/null | grep -q 'Connected: yes'; then
    was_connected=true
fi
if btmgmt --index "$HCI_INDEX" info | grep -q 'current settings:.* br/edr'; then
    was_bredr_enabled=true
fi

if "$was_bredr_enabled"; then
    echo "Administrator access is required to temporarily disable classic Bluetooth."
    sudo -v
fi

restore() {
    if "$was_bredr_enabled"; then
        echo "Re-enabling classic Bluetooth..."
        sudo btmgmt --index "$HCI_INDEX" bredr on >/dev/null 2>&1 || true
    fi

    if "$was_connected"; then
        echo "Restoring the classic audio connection..."
        bluetoothctl connect "$CLASSIC_ADDRESS" || \
            echo "Could not reconnect automatically; reconnect it from your desktop Bluetooth menu."
    fi
}
trap restore EXIT INT TERM

echo "Ensuring the local Bluetooth adapter is powered on..."
bluetoothctl power on >/dev/null

# The headset immediately reconnects its classic audio endpoint when it powers on.
# Disable BR/EDR at the controller level so LE scanning remains available but that
# automatic connection cannot occupy the headset. The trap restores it on exit.
if "$was_bredr_enabled"; then
    echo "Temporarily disabling classic Bluetooth audio (BLE remains on)..."
    sudo btmgmt --index "$HCI_INDEX" bredr off >/dev/null
fi

echo
echo "Scanning for $SCAN_SECONDS seconds. Keep phone Bluetooth off and power the headset on in Bluetooth mode now."
bluetoothctl --timeout "$SCAN_SECONDS" scan on || true

echo
echo "Scan result:"
if bluetoothctl devices | grep -qi "$BLE_ADDRESS"; then
    bluetoothctl info "$BLE_ADDRESS"
    echo
    echo "Found the BlackShark BLE control endpoint ($BLE_ADDRESS)."
    echo "Send this output to the project maintainer before pairing it manually."
else
    echo "The BlackShark BLE control endpoint ($BLE_ADDRESS) was not advertised."
    echo "The normal audio connection remains usable, but ANC/EQ control cannot be added until this endpoint is visible to BlueZ."
fi
