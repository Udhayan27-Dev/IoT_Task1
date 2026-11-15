# Task 2: Adafruit IO Dashboard & MQTT Relay Control

A bare-metal asynchronous embedded Rust application for the **ESP32-C3** (RISC-V `riscv32imc-unknown-none-elf`) that connects to **Adafruit IO** over **MQTT** to remotely switch an AC light bulb/appliance via an active-LOW relay module.

Built using the bare-metal (`no_std`) Rust ecosystem with **`esp-hal`**, **`esp-rtos`**, **`esp-radio`**, and the **`embassy`** async runtime.

---

## Overview

- **MCU**: ESP32-C3 (RISC-V bare-metal `riscv32imc-unknown-none-elf`)
- **Language**: Embedded Rust (`no_std`) with memory safety and zero OS overhead
- **Cloud Broker**: Adafruit IO MQTT broker (`io.adafruit.com:1883`)
- **Protocol**: MQTT 3.1.1 implementation over raw async TCP (`embassy-net`)
- **Heartbeat**: Automatic keep-alive `PINGREQ` every 45 seconds
- **Hardware Output**: GPIO 10 wired to a 5V/3.3V optocoupled relay module (Active-LOW)

---

## Architecture & Data Flow

```
[ Adafruit IO Dashboard ]
           │
           │  Toggle Switch (ON / OFF)
           ▼
[ Adafruit IO Cloud Broker ] (io.adafruit.com:1883)
           │
           │  MQTT PUBLISH (topic: <user>/feeds/bulb)
           ▼
[ ESP32-C3 Async TCP Socket ] (embassy-net)
           │
           │  Decodes MQTT 3.1.1 Packet
           ▼
[ Critical Section Mutex ]
           │
           ├─── pin.set_low()  ──► Relay ENERGIZED (Bulb ON)
           │
           └─── pin.set_high() ──► Relay DE-ENERGIZED (Bulb OFF)
```

---

## Hardware Wiring

| ESP32-C3 Pin | Relay Module Pin | Description |
| :--- | :--- | :--- |
| `GPIO 10` | `IN` | Control signal (Active-LOW: LOW = ON, HIGH = OFF) |
| `5V / VIN` | `VCC` | Relay coil power |
| `GND` | `GND` | Common ground reference |

> **Safety Warning**: The relay switches AC mains voltage (230V AC). Ensure proper insulation and never touch live terminals while plugged in.

---

## Project Structure

```
relay/
├── .cargo/
│   └── config.toml           # RISC-V build target & espflash runner
├── src/
│   ├── bin/
│   │   └── main.rs           # Entry point, Wi-Fi task, async MQTT task & relay logic
│   └── lib.rs                # Library root
├── build.rs                  # Espressif linker script bindings
├── Cargo.toml                # Dependencies: esp-hal, esp-radio, embassy-net, etc.
└── rust-toolchain.toml       # Toolchain specification (riscv32imc-unknown-none-elf)
```

---

## Building and Flashing

### Prerequisites

1. Install the Rust RISC-V target:
   ```bash
   rustup target add riscv32imc-unknown-none-elf
   rustup component add rust-src
   ```
2. Install `espflash`:
   ```bash
   cargo install espflash --locked
   ```

### Configuration

Edit `src/bin/main.rs` to configure your credentials:
```rust
const WIFI_SSID: &str = "YOUR_WIFI_SSID";
const WIFI_PASS: &str = "YOUR_WIFI_PASSWORD";

const AIO_USERNAME: &str = "YOUR_ADAFRUIT_IO_USERNAME";
const AIO_KEY: &str      = "YOUR_AIO_KEY";
const AIO_FEED_KEY: &str = "bulb";
```

### Flash to ESP32-C3

Connect your board via USB and run:
```bash
cargo run --release
```

The serial monitor displays:
```text
Connecting to Wi-Fi...
Wi-Fi connected to AP!
Wi-Fi Connected! IP: 192.168.1.120
Connecting to Adafruit IO MQTT (port 1883)...
TCP Connected! Sending MQTT CONNECT...
MQTT Connected successfully to Adafruit IO!
Subscribed to feed 'uyya/feeds/bulb'! Awaiting dashboard toggle...
>> MQTT Received: ON -> Bulb turned ON <<
>> MQTT Received: OFF -> Bulb turned OFF <<
```
