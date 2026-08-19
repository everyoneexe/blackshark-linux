# blackshark-linux

Standalone Linux control tools for the **Razer BlackShark V3 X HyperSpeed (Xbox)**
wireless headset.

No Razer Synapse, no Windows. The device's USB HID protocol was reverse-engineered;
every command is byte-for-byte verified against what Synapse actually sends.

Controls EQ presets, custom 10-band EQ, sidetone (mic monitoring), mic noise
cancellation, sleep timer, power saving, and battery reading.

## Architecture

```
blackshark-tray (Qt/C++)  ──┐
settings window (Qt GUI)  ──┤  D-Bus: net.blackshark1 (session bus)
blackshark      (Rust CLI) ─┘  path: /net/blackshark1/Headset
                            │
                     blacksharkd  (Rust, systemd user service)
                            │  owns the HID device exclusively
                     /dev/hidraw*  (hidapi)
                            │
                     BlackShark V3 X (USB 1532:0A5B)
```

The daemon owns the HID device exclusively. All other tools talk to it over D-Bus.
Nothing but the daemon touches `/dev/hidraw*`, so the tray, GUI, and CLI never
conflict.

## Features

- **EQ presets** — Default, Game, Movie, Music
- **Custom EQ** — 10 bands, -6..+6 dB (31 Hz .. 16 kHz)
- **Sidetone** — mic monitoring level (0-15)
- **Noise cancellation** — mic noise cancellation on/off
- **Sleep timer** — auto-off timeout in minutes
- **Power saving** — on/off
- **Battery** — percentage, polled every 5 minutes, pushed over D-Bus
- **Settings persist** — saved to `~/.config/blackshark/config.toml`, restored on
  reconnect and boot (the device firmware forgets most settings)
- **System tray** — plain headset icon, battery %, quick toggles, EQ/sidetone submenus
- **GUI** — simple settings panel with live status
- **CLI** — scriptable control with JSON status output

## Requirements

- Linux with systemd (user session) and D-Bus
- Rust (stable) — https://rustup.rs — for the daemon and CLI
- CMake + Qt6 (or Qt5) with Widgets, DBus, Svg — for the tray/GUI
- `libudev` (hidapi dependency)
  - Debian/Ubuntu: `sudo apt install libudev-dev pkg-config cmake qt6-base-dev`
  - Fedora: `sudo dnf install systemd-devel cmake qt6-qtbase-devel`
  - Arch: `sudo pacman -S cmake qt6-base`

> **Firmware note:** if the daemon sees the dongle but cannot talk to the headset,
> update the headset/dongle firmware via Razer Synapse on Windows first.

## Quick install

```sh
./install.sh
```

This builds everything, installs the binaries to `~/.local/bin`, installs the
udev rule (asks for `sudo`), enables the `blacksharkd` systemd user service, and
adds the tray to autostart.

If you are not in the `plugdev` group:

```sh
sudo usermod -aG plugdev $USER   # then log out and back in
```

## Getting started

After install, plug in the USB dongle. Then:

```sh
# 1. Check the daemon is running
systemctl --user status blacksharkd

# 2. Verify the headset is detected
blackshark status

# 3. Start the tray (auto-starts on next login)
blackshark-tray &
```

The tray icon shows a plain headset. Left-click opens Settings; right-click opens
the menu with battery %, EQ presets, sidetone, noise cancellation, power saving,
sleep timer, and daemon controls.

## Usage (CLI)

```sh
blackshark status                          # human-readable status
blackshark status --json                   # JSON output (for waybar/scripts)
blackshark battery                         # battery percentage
blackshark eq-preset game                  # default | game | movie | music
blackshark eq 1 2 3 4 5 6 -1 -2 -3 -4      # custom EQ, 31Hz..16kHz (-6..+6)
blackshark sidetone 8                      # 0..15
blackshark noise-cancel on                 # on | off
blackshark sleep 30                        # minutes
blackshark power-save on                   # on | off
blackshark apply                           # re-apply saved config to the device
```

Every setting command is applied to the device **and** saved to config, so it is
restored automatically on reconnect and boot.

## Repository layout

```
blackshark-rs/                 Rust daemon + CLI
  src/protocol.rs              HID report format, CRC, command builders + tests
  src/device.rs                hidapi open/send/recv
  src/config.rs                TOML config load/save/apply
  src/daemon.rs                blacksharkd: HID owner, D-Bus service, battery poll
  src/main.rs                  blackshark: CLI client (D-Bus)
  systemd/blacksharkd.service  systemd user unit
  99-blackshark.rules          udev rule
  config.example.toml          example config
blackshark-qt/                 Qt/C++ tray + GUI
  main.cpp                     entry point
  HeadsetClient.{h,cpp}        shared D-Bus client
  TrayApp.{h,cpp}              system tray
  SettingsWindow.{h,cpp}       settings window
  assets/headset.svg           tray icon
  pkg/blackshark-tray.desktop  autostart entry
  CMakeLists.txt
install.sh                     one-shot build + install
PROTOCOL.md                    full reverse-engineered protocol documentation
captures/                      raw USBPcap captures (protocol evidence)
```

## Testing the protocol

```sh
cd blackshark-rs
cargo test        # verifies generated report bytes against real Synapse captures
```

## Device info

- USB VID/PID: `0x1532` / `0x0A5B`
- HID reports: 64 bytes, report ID `0x07`
- CRC: `XOR(byte[3..=60])`
- Protocol: custom Razer HID (not HID++ or OpenRazer-compatible)

See `PROTOCOL.md` for the full command reference.

## Credits

The D-Bus daemon/client layout was initially inspired by
[RiskRunner0/blackshark-linux](https://github.com/RiskRunner0/blackshark-linux).
This is an independent implementation for the BlackShark V3 X HyperSpeed (Xbox)
(`1532:0A5B`); it has its own HID protocol implementation, device support, and
release workflow.
