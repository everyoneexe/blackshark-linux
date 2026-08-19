# blackshark-rs

Rust part of **blackshark-linux**: the daemon (`blacksharkd`) and the CLI client
(`blackshark`).

- `blacksharkd` — owns the HID device, exposes the `net.blackshark1` D-Bus
  service on the session bus, polls the battery, and persists settings to
  `~/.config/blackshark/config.toml`.
- `blackshark` — CLI client that talks to the daemon over D-Bus.

See the top-level `README.md` for the full picture (architecture, install, Qt tray)
and `PROTOCOL.md` for the reverse-engineered protocol.

## Build

```sh
cargo build --release
# binaries: target/release/blacksharkd, target/release/blackshark
```

Requires `libudev` (hidapi dependency):
- Debian/Ubuntu: `sudo apt install libudev-dev pkg-config`
- Fedora: `sudo dnf install systemd-devel`
- Arch: already available

## Test

```sh
cargo test        # verifies generated report bytes against real Synapse captures
```

## CLI

```sh
blackshark status [--json]
blackshark battery
blackshark eq-preset <default|game|movie|music>
blackshark eq <10 dB values, -6..6>     # 31Hz..16kHz
blackshark sidetone <0-15>
blackshark noise-cancel <on|off>
blackshark sleep <minutes>
blackshark power-save <on|off>
blackshark apply                         # re-apply saved config
```
