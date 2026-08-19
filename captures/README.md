# USB HID Captures

Raw USBPcap captures of Razer Synapse talking to the BlackShark V3 X HyperSpeed
(Xbox) headset (`1532:0A5B`). These are the evidence behind `PROTOCOL.md`: every
decoded command was extracted from these files and verified byte-for-byte.

Open with Wireshark, or parse with:

```sh
tshark -r <file>.pcap -Y "usb.transfer_type==0x02 && usb.endpoint_address.direction==0 && frame.len>60" -T fields -e usb.data_fragment
```

| File | What it shows |
|------|---------------|
| `baseline.pcap`        | Idle Synapse polling traffic (first look at the report format) |
| `eq_presets.pcap`      | EQ preset switching (Default/Game/Movie/Music) — command 0x18/0x00 |
| `eq_custom_bands.pcap` | First custom EQ band edits — command 0x18/0x01 |
| `eq_ladder.pcap`       | Bands set in a staircase to map each band's byte position |
| `sidetone.pcap`        | Mic monitoring / sidetone levels — command 0x18/0x03 |
| `mic.pcap`             | Mic tab traffic (mic volume is software-side; sidetone is the vendor command) |
| `mic_enh_nc.pcap`      | Mic noise cancellation toggle — command 0x18/0x0f |
| `power2.pcap`          | Sleep timer + power saving — commands 0x07/0x03 and 0x07/0x08 |
| `synapse_restart.pcap` | Full startup handshake; battery response decoded here (0x07/0x80 = 92%) |
| `battery3.pcap`        | Extra battery-related capture |

See `../PROTOCOL.md` for the decoded command reference.
