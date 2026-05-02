# DICE Node — Hardware Reference & Setup Guide

ESP32-S3-N16R8 firmware for the DICE VRF oracle network.

---

## 1. Bill of Materials (BOM)

| Component | Spec | Notes |
|-----------|------|-------|
| MCU Board | ESP32-S3-N16R8 DevKit | 16 MB Flash, 8 MB PSRAM, USB-JTAG built-in |
| Status LED | WS2812 (onboard) | Addressable RGB on GPIO48 — no external LED needed |
| Entropy Source | Floating ADC pin | ADC1_CH0 = GPIO1, **leave unconnected** |
| USB Cable | USB-C | For flashing and serial monitor |
| Power | 5V USB or 3.3V regulated | Normal operating current ~120 mA (WiFi active) |

No external components required for a basic node. The ESP32-S3-N16R8 DevKit has everything onboard.

---

## 2. Pin Mapping

| GPIO | Function | Direction | Notes |
|------|----------|-----------|-------|
| 48 | WS2812 status LED | Output | Onboard addressable RGB LED (RMT driver, 10 MHz) |
| 1 | ADC entropy source | Input | ADC1_CHANNEL_0 — **must be floating** (no connection) |
| 19/20 | USB-JTAG | Bidirectional | Built-in USB for flashing + serial (ESP32-S3 native) |

All other GPIO pins are unused by the firmware and can be left in default state.

> **Important:** GPIO1 must remain **unconnected** — it samples thermal/EMI noise from the floating pin as an additional entropy source mixed into the VRF seed. Connecting anything to GPIO1 degrades entropy quality.

---

## 3. NVS Data Map

All persistent data is stored in NVS namespace `"dice"` (encrypted when flash encryption is enabled).

| Key | Type | Set By | Description |
|-----|------|--------|-------------|
| `wifi_ssid` | string (max 32) | Captive portal | WiFi network name |
| `wifi_pass` | string (max 64) | Captive portal | WiFi password |
| `sol_wallet` | string (max 44) | Captive portal | Operator's Solana wallet address |
| `coordinator_uri` | string (max 255) | Provisioning script | WSS endpoint, e.g. `wss://coord.dice.io/ws` |
| `priv_key_der` | blob (max 512) | Provisioning script | ECDSA secp256k1 private key (DER format) |
| `client_cert_pem` | string | Provisioning script | Device mTLS certificate (PEM) |
| `client_key_pem` | string | Provisioning script | Device mTLS private key (PEM) |
| `ca_cert_pem` | string | Provisioning script | CA certificate for coordinator verification (PEM) |

**Captive portal sets:** `wifi_ssid`, `wifi_pass`, `sol_wallet`
**Provisioning script sets:** `coordinator_uri`, `priv_key_der`, `client_cert_pem`, `client_key_pem`, `ca_cert_pem`

---

## 4. Flash Partition Layout

From `partitions.csv` — no OTA partitions (firmware is immutable after provisioning):

| Name | Type | Offset | Size | Purpose |
|------|------|--------|------|---------|
| nvs | data/nvs | 0x9000 | 24 KB | Non-volatile storage (keys, WiFi creds, certs) |
| phy_init | data/phy | 0xF000 | 4 KB | WiFi/BT PHY calibration data |
| factory | app/factory | 0x10000 | 1500 KB | Application firmware (single image, no OTA) |

Total flash usage: ~1.5 MB of 16 MB. Remaining space is reserved for flash encryption overhead.

---

## 5. Security Configuration

Enforced via eFuses (irreversible once burned):

| Feature | Setting | Purpose |
|---------|---------|---------|
| Secure Boot v2 | Enabled | Only signed firmware boots — prevents tampering |
| Flash Encryption | Release mode | All flash contents encrypted — prevents readout |
| NVS Encryption | Enabled | NVS partition encrypted — protects keys at rest |
| No OTA | Firmware immutable | No remote update path — eliminates supply chain attack vector |

> **Warning:** Secure Boot and Flash Encryption eFuses are **one-time burn**. Once enabled in Release mode, the device cannot be reflashed with unsigned firmware. Only burn eFuses on final production firmware.

---

## 6. Build Environment Setup

### Prerequisites

- ESP-IDF v5.x (tested with v5.2)
- Python 3.8+
- CMake 3.16+
- Linux or macOS recommended (WSL2 works on Windows)

### Install ESP-IDF

```bash
mkdir -p ~/esp && cd ~/esp
git clone -b v5.2 --recursive https://github.com/espressif/esp-idf.git
cd esp-idf && ./install.sh esp32s3
source export.sh
```

### Build

```bash
cd firmware
idf.py set-target esp32s3
idf.py build
```

Build output lands in `firmware/build/`. Key artifacts:
- `dice_firmware.bin` — application binary
- `bootloader/bootloader.bin` — second-stage bootloader
- `partition_table/partition-table.bin` — partition table

---

## 7. Flashing

### First flash (development, no Secure Boot)

Connect the ESP32-S3 via USB-C. The built-in USB-JTAG interface appears as a serial port.

```bash
cd firmware
idf.py -p /dev/ttyACM0 flash monitor
```

On macOS the port is typically `/dev/cu.usbmodem*`. On Windows (WSL2) it maps to `/dev/ttyS*` or use native `COM*` with `esptool.py`.

### Production flash (with Secure Boot + Flash Encryption)

**This is irreversible. Only do this on final production firmware.**

```bash
# 1. Build with security features
idf.py build

# 2. Flash bootloader, partition table, and app
idf.py -p /dev/ttyACM0 flash

# 3. Burn Secure Boot key (generates and burns to eFuse)
espefuse.py -p /dev/ttyACM0 burn_key \
    BLOCK_KEY0 secure_boot_signing_key.pem SECURE_BOOT_DIGEST

# 4. Enable flash encryption (burns eFuse — IRREVERSIBLE)
espefuse.py -p /dev/ttyACM0 burn_efuse FLASH_CRYPT_CNT 0x7

# 5. Enable Secure Boot (burns eFuse — IRREVERSIBLE)
espefuse.py -p /dev/ttyACM0 burn_efuse ABS_DONE_0
```

> After burning eFuses, the device will only boot signed + encrypted firmware. Keep `secure_boot_signing_key.pem` safe — losing it means the device is bricked.

---

## 8. Factory Provisioning

After flashing firmware, each device needs its cryptographic identity provisioned into NVS.

### What gets provisioned

1. **ECDSA secp256k1 keypair** — device identity for VRF signing (`priv_key_der`)
2. **mTLS certificates** — for authenticated WebSocket to coordinator (`client_cert_pem`, `client_key_pem`, `ca_cert_pem`)
3. **Coordinator URI** — WebSocket endpoint (`coordinator_uri`)

### Provisioning flow

```
[Factory workstation]
    |
    |-- 1. Generate secp256k1 keypair
    |-- 2. Sign device cert with DICE CA
    |-- 3. Write to NVS via esptool/nvs_partition_gen
    |-- 4. Record device public key in coordinator registry
    |
    v
[Device boots → captive portal (blue LED)]
    |
    |-- User connects to "DICE-XXXX" WiFi
    |-- Opens http://192.168.4.1
    |-- Enters WiFi creds + Solana wallet
    |-- Device saves to NVS, reboots
    |
    v
[Device boots → normal mode (green LED)]
```

### Manual NVS provisioning (development)

Use `nvs_partition_gen.py` from ESP-IDF to create a pre-populated NVS partition:

```bash
# Create CSV with provisioning data
cat > nvs_data.csv << 'EOF'
key,type,encoding,value
dice,namespace,,
coordinator_uri,data,string,wss://coordinator.dice.io/ws
priv_key_der,data,binary,device_key.der
client_cert_pem,file,string,device_cert.pem
client_key_pem,file,string,device_key.pem
ca_cert_pem,file,string,ca_cert.pem
EOF

# Generate NVS partition binary
python $IDF_PATH/components/nvs_flash/nvs_partition_generator/nvs_partition_gen.py \
    generate nvs_data.csv nvs_provisioned.bin 0x6000

# Flash just the NVS partition
esptool.py -p /dev/ttyACM0 write_flash 0x9000 nvs_provisioned.bin
```

---

## 9. LED Status Reference

The onboard WS2812 LED on GPIO48 indicates device state:

| Color | Pattern | State | Meaning |
|-------|---------|-------|---------|
| Blue | Solid | Setup | Captive portal active — connect to "DICE-XXXX" WiFi |
| Yellow | Solid | Connecting | WiFi connecting / crypto init / WebSocket connecting |
| Green | Solid | Online | Connected to coordinator, waiting for VRF jobs |
| Green | Blinking (500ms) | Active | Currently participating in a commit-reveal round |
| Red | Solid | Error | Fatal error — crypto failure, entropy test failed, etc. |
| Off | — | Off | Device powered down or LED driver failed to init |

---

## 10. Boot Sequence

```
Power On
  │
  ├── NVS flash init
  ├── LED driver init (GPIO48)
  │
  ├── Check NVS for "wifi_ssid"
  │     │
  │     ├── NOT FOUND ──→ 🔵 Captive Portal
  │     │                    WiFi AP: "DICE-XXXX"
  │     │                    HTTP: 192.168.4.1
  │     │                    DNS: all queries → 192.168.4.1
  │     │                    User configures → NVS save → reboot
  │     │
  │     └── FOUND ──→ Normal Boot
  │
  ├── 🟡 Load secp256k1 key from NVS (priv_key_der)
  ├── 🟡 Entropy self-test (10 SHA-256 samples, uniqueness check)
  ├── 🟡 WiFi station connect (from NVS creds)
  │     │
  │     ├── FAIL (10 retries) → Clear WiFi creds → 🔴 → Reboot → Captive Portal
  │     └── OK → Continue
  │
  ├── 🟡 WebSocket connect (mTLS) to coordinator
  │     │
  │     ├── FAIL → 🔴 → Halt
  │     └── OK → 🟢 Online
  │
  ├── Start heartbeat timer (25s interval)
  │
  └── Main loop (60s status log, watchdog feed)
        └── On JobAssignment → 💚 blink → commit-reveal → 🟢 solid
```

---

## 11. Entropy Sources

The firmware mixes three independent entropy sources via XOR, then finalizes with SHA-256:

| Source | Bits | Origin | Failure Mode |
|--------|------|--------|-------------|
| Hardware TRNG | 256 | ESP32-S3 `esp_fill_random()` — ring oscillator based | Fatal (chip defect) |
| ADC noise | 32 (folded to 256) | Floating GPIO1 via ADC1_CH0, 8 samples XOR-folded | Non-fatal (falls back to tick count) |
| Timing jitter | 96 | FreeRTOS tick count + `esp_timer` microseconds | Always available |

The entropy self-test at boot generates 10 samples and verifies:
- All 10 are unique (no two identical)
- First sample is not all zeros

If the self-test fails, the device halts (red LED) and will not participate in VRF rounds.

---

## 12. First-Unit Bring-Up Checklist

Use this for verifying a newly assembled device.

- [ ] **Power:** Connect USB-C, verify 3.3V rail (if exposed test point)
- [ ] **Serial:** Open serial monitor at 115200 baud — confirm ESP-IDF boot log
- [ ] **Flash firmware:** `idf.py flash` succeeds without errors
- [ ] **LED init:** Blue LED appears within 2 seconds of power-on (first boot)
- [ ] **WiFi AP:** Scan for "DICE-XXXX" network on phone/laptop
- [ ] **Captive portal:** Connect to AP, browser auto-opens or navigate to `192.168.4.1`
- [ ] **Setup page:** Page renders with dark theme, device ID shown
- [ ] **Configure:** Enter WiFi SSID, password, and a test Solana wallet address
- [ ] **Save & reboot:** "Saved! Rebooting..." message, device reboots within 3s
- [ ] **WiFi connect:** Yellow LED → serial log shows "Got IP: x.x.x.x"
- [ ] **Crypto init:** Serial log shows "Crypto context initialised (secp256k1)"
- [ ] **Entropy test:** Serial log shows "Entropy self-test PASSED"
- [ ] **WebSocket:** Green LED = connected to coordinator (requires provisioned certs)
- [ ] **WiFi fail recovery:** Enter wrong WiFi password → 10 retries → red LED → reboots to captive portal
- [ ] **ADC entropy pin:** Verify GPIO1 is floating (no solder bridges, no trace connections)

---

## 13. Troubleshooting

| Symptom | Likely Cause | Fix |
|---------|-------------|-----|
| No LED on boot | LED driver init failed, or GPIO48 not connected | Check board variant — some devkits use GPIO38 instead |
| Blue LED stays on | Device in captive portal / not provisioned | Connect to "DICE-XXXX" WiFi and complete setup |
| Yellow LED stays on | WiFi connecting or crypto init stuck | Check WiFi credentials, verify NVS has `priv_key_der` |
| Red LED | Fatal error during boot | Check serial monitor for specific error message |
| Red LED after yellow | WiFi connection failed | Wrong SSID/password — device will reboot to captive portal |
| "Crypto init FAILED" | Missing or corrupt `priv_key_der` in NVS | Re-run provisioning script to write key to NVS |
| "Entropy self-test FAILED" | GPIO1 is grounded or ADC + TRNG both failing | Check GPIO1 is floating; if persistent, possible chip defect |
| WebSocket won't connect | Missing mTLS certs or wrong coordinator URI | Verify `client_cert_pem`, `client_key_pem`, `ca_cert_pem`, `coordinator_uri` in NVS |
| Captive portal page won't load | DNS redirect not working | Try navigating directly to `http://192.168.4.1` |
| `esptool.py` can't find port | USB driver issue | Install CP210x or CH340 driver; ESP32-S3 native USB needs no driver on Linux |

---

## 14. Key Files Reference

| File | Purpose |
|------|---------|
| `firmware/CMakeLists.txt` | Top-level project CMake |
| `firmware/main/CMakeLists.txt` | Source files and component dependencies |
| `firmware/sdkconfig.defaults` | SDK config: target chip, security, crypto, WiFi, LED, HTTP |
| `firmware/partitions.csv` | Flash partition table (NVS + factory app, no OTA) |
| `firmware/main/app_main.c` | Entry point: boot sequence, WiFi, main loop |
| `firmware/main/captive_portal.c/h` | First-boot WiFi AP + HTTP setup page + DNS redirect |
| `firmware/main/led_status.c/h` | WS2812 LED driver with 6 status modes |
| `firmware/main/entropy.c/h` | 3-source entropy mixing (TRNG + ADC + timing) |
| `firmware/main/crypto.c/h` | ECDSA secp256k1 key loading, signing, SHA-256 |
| `firmware/main/websocket_client.c/h` | mTLS WebSocket client with reconnection backoff |
| `firmware/main/heartbeat.c/h` | 25-second periodic heartbeat to coordinator |
| `firmware/main/commit_reveal.c/h` | VRF commit-reveal protocol handler |
| `firmware/main/dice_protocol/` | Wire protocol (message types, serialization) |
