# Task 1: ESP32-C3 Web Server & HTML LED Control

A bare-metal asynchronous embedded Rust application running on the **ESP32-C3** RISC-V microcontroller. It hosts an embedded HTTP web server on the local Wi-Fi network, allowing users to switch an LED on and off from any web browser (phone or PC).

Built using the modern bare-metal (`no_std`) Rust ecosystem with **`esp-hal`**, **`esp-rtos`**, **`esp-radio`**, and the **`embassy`** async runtime.

---

## Overview

- **MCU**: ESP32-C3 (RISC-V `riscv32imc-unknown-none-elf`)
- **Language**: Embedded Rust (`no_std`)
- **Networking**: Wi-Fi Station mode via `esp-radio` and DHCP via `embassy-net`
- **Application Server**: Async TCP socket listener on port 80 serving an interactive HTML dashboard
- **Hardware Control**: Direct GPIO 10 push-pull output toggling an LED

---

## Architecture & Data Flow

```
[ Smartphone / Browser ]
           │
           │  HTTP GET /on, /off, /
           ▼
[ Wi-Fi Station Interface ]
           │
           │  embassy-net TCP Socket (Port 80)
           ▼
[ Async Web Task (Rust) ]
           │
           ├─── GPIO 10 HIGH / LOW ───► [ LED + Resistor ]
           │
           └─── HTTP 200 / HTML Response ───► [ Web Browser UI ]
```

---

## Hardware Wiring

| Component | Pin on ESP32-C3 | Description |
| :--- | :--- | :--- |
| **LED Anode (+)** | `GPIO 10` | Digital Output (High = ON, Low = OFF) |
| **LED Cathode (-)** | `GND` | Ground via 220 Ω current-limiting resistor |

---

## Project Structure

```
led-web-control/
├── .cargo/
│   └── config.toml           # RISC-V build target & espflash runner
├── src/
│   ├── bin/
│   │   └── main.rs           # Main entry point, connection task, net task, web server task
│   └── lib.rs                # Library root
├── build.rs                  # Espressif linker script bindings
├── Cargo.toml                # Dependencies: esp-hal, esp-radio, embassy-net, etc.
└── rust-toolchain.toml       # Toolchain specification (riscv32imc-unknown-none-elf)
```

---

## Building and Flashing

### Prerequisites

1. Install the Rust RISC-V bare-metal target:
   ```bash
   rustup target add riscv32imc-unknown-none-elf
   rustup component add rust-src
   ```
2. Install `espflash`:
   ```bash
   cargo install espflash --locked
   ```

### Configuration

Edit `src/bin/main.rs` to configure your 2.4 GHz Wi-Fi credentials:
```rust
const SSID: &str = "YOUR_WIFI_SSID";
const PASSWORD: &str = "YOUR_WIFI_PASSWORD";
```

### Flash to ESP32-C3

Connect your board via USB and run:
```bash
cargo run --release
```

The serial monitor will launch at 115200 baud, showing:
```text
Embassy initialized!
start connection task
Connecting...
Connected! ...
Waiting for IP address...
Got IP: 192.168.1.50
Listening on port 80...
```

Open `http://<printed-ip>/` in your browser to toggle the LED!
