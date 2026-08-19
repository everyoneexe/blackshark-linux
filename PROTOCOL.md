# Razer BlackShark V3 X HyperSpeed (Xbox) — Reverse Engineering Notes

## Device Identity
- **VID**: `0x1532` (Razer)
- **PID**: `0x0A5B`
- USB Composite Device
  - **MI_00**: HID control interface (Synapse commands go through here)
    - COL01: consumer control
    - COL02: "phone" (call control — Xbox/gaming)
    - COL03, COL04: vendor-defined (COMMAND CHANNELS)
  - **MI_01**: USB Audio (Speakers + Microphone)

## Transport
- Commands are sent via **USB CONTROL transfer** (HID SET_REPORT / GET_REPORT)
- SET_REPORT (out): Synapse -> device (write command/setting)
- GET_REPORT (in): device -> Synapse (read status)
- Report ID: `0x07`
- Total report size: **64 bytes**

## Report Structure (64 bytes)

```
Offset  Size   Field             Description
------  -----  ----------------  ------------------------------------------
[0]     1      report_id         Always 0x07
[1]     1      status            0x00 (request); may differ in response
[2]     1      transaction_id    Incrementing counter (00, 01, 02, ... +1 per command)
[3]     1      ?                 0x00 (remaining packets high?)
[4]     1      ?                 0x00
[5]     1      ?                 0x00 (protocol type?)
[6]     1      data_size         Argument data size (0x02, 0x04, ...)
[7]     1      command_class     0x18 = Audio/EQ, 0x07 = Power/Battery
[8]     1      command_id        SET vs GET (high bit set = read)
[9..]   N      arguments         Command-specific arguments
[62]    1      crc               XOR of byte[3]..byte[60]
[63]    1      reserved          0x00
```

### CRC Computation (VERIFIED)
```
crc = 0
for i in 3..=60:
    crc ^= byte[i]
byte[62] = crc
```

## Decoded Commands

### Set EQ Preset (command_class=0x18, command_id=0x00)
```
07 00 TT 00 00 00 04 18 00 00 01 00 VV [00...] CRC 00
                                     └─ VV = preset value
```
- data_size = 0x04
- arguments (byte[9..12]) = `00 01 00 VV`
- VV preset values:
  - `0x00` = DEFAULT (VERIFIED)
  - `0x01` = GAME
  - `0x02` = MOVIE
  - `0x03` = MUSIC

### Read EQ Preset (command_class=0x18, command_id=0x81)
```
07 00 TT 00 00 00 02 18 81 00 01 00 00 [00...] CRC 00
```
- data_size = 0x02, command_id 0x81 (GET)

### Custom EQ Band Values (command_class=0x18, command_id=0x01)
```
07 00 TT 00 00 00 0c 18 01 00 01 B1 B2 B3 ... [00...] CRC 00
```
- data_size = 0x0c (12)
- arguments (from byte[9]):
  - byte[9]  = 0x00
  - byte[10] = 0x01  (fixed prefix)
  - byte[11] = band 1 (31 Hz) value
  - byte[12..] = remaining bands (1 byte per band)
- **10 bands, starting at byte[11], left to right (VERIFIED):**
  - byte[11] = 31 Hz    (band 1)
  - byte[12] = 63 Hz    (band 2)
  - byte[13] = 125 Hz   (band 3)
  - byte[14] = 250 Hz   (band 4)
  - byte[15] = 500 Hz   (band 5)
  - byte[16] = 1 kHz    (band 6)
  - byte[17] = 2 kHz    (band 7)
  - byte[18] = 4 kHz    (band 8)
  - byte[19] = 8 kHz    (band 9)
  - byte[20] = 16 kHz   (band 10)
- **Band value encoding = signed-magnitude (VERIFIED):**
  - 0 dB        = 0x00
  - Positive dB = direct value: +1=0x01 ... +6=0x06
  - Negative dB = 0x80 | abs(dB): -1=0x81, -3=0x83, -6=0x86
- Range: -6 dB .. +6 dB

### Mic Monitoring / Sidetone (command_class=0x18, command_id=0x03)
```
07 00 TT 00 00 00 03 18 03 00 03 VV [00...] CRC 00
```
- data_size = 0x03
- args (byte[9..11]) = `00 03 VV`
  - byte[9]  = 0x00
  - byte[10] = 0x03 (fixed)
  - byte[11] = VV = sidetone level
- **Value range: 0x00 (off) .. 0x0F (max)** = 16 steps (0-15)
- Observed: 0x00, 0x01, 0x07, 0x0F

### Mic Noise Cancellation (command_class=0x18, command_id=0x0f)
```
07 00 TT 00 00 00 02 18 0f 00 VV [00...] CRC 00
```
- data_size = 0x02
- args (byte[9]) = VV = 0x00 (off) / 0x01 (on)
- Simple on/off toggle

### Power Saving Toggle (command_class=0x07, command_id=0x08)
```
07 00 TT 00 00 00 02 07 08 00 VV [00...] CRC 00
```
- data_size = 0x02
- args (byte[9]) = VV = 0x00 (off) / 0x01 (on)

### Sleep Timer (command_class=0x07, command_id=0x03)
```
07 00 TT 00 00 00 02 07 03 HH LL [00...] CRC 00
```
- data_size = 0x02
- args (byte[8..9]) = 16-bit BIG-ENDIAN seconds
- **VERIFIED:**
  - 30 min = 1800s = 0x0708
  - 45 min = 2700s = 0x0A8C
  - 60 min = 3600s = 0x0E10

### Read Battery Level (command_class=0x07, command_id=0x80) [GET]
```
Query (OUT):    07 00 TT 00 00 00 02 07 80 00 00 [00...] CRC 00
Response (IN):  .. .. .. .. .. .. 02 07 80 00 VV [00...] CRC 00
```
- VV = battery percentage (0x00..0x64 = 0..100)
- **VERIFIED: 0x5C = 92 = 92% (exactly matched the on-screen value)**

### Other GET commands (class 0x07, not yet decoded)
- 0x83 = read sleep timer (GET of 0x03, presumed)
- 0x84 = ? (polled frequently)
- 0x88 = read power saving state (GET of 0x08, presumed)

### Other GET commands (class 0x18, EQ/audio)
- 0x80 = ? , 0x81 = read EQ preset, 0x83/0x84/0x85/0x8F = GETs of related settings

## SOFTWARE-SIDE (no port needed — already works on Linux)
- VOLUME toggle: no vendor command is sent to the device; it's system audio /
  software side. Only a 1-byte output report (01=on, 00=off) that triggers the
  Windows audio chain. Handled by pipewire/pavucontrol on Linux. SKIP.
- 7.1 Surround: software-side spatial audio, nothing is sent to the device.
  Done on Linux with easyeffects, etc. SKIP.

## Status (device firmware features)
- [x] DEFAULT preset value = 0x00
- [x] Custom EQ band values — FULLY DECODED (10 bands, byte[11..20], signed-mag)
- [x] Volume — determined to be software-side, no port needed
- [x] 7.1 — software-side, no port needed
- [x] Sidetone / mic monitoring (0x18/0x03)
- [x] Mic noise cancellation (0x18/0x0f)
- [x] Sleep timer (0x07/0x03, big-endian seconds)
- [x] Power saving toggle (0x07/0x08)
- [x] Battery read (0x07/0x80, verified = 92%)
