# IoT Systems & Embedded Engineering Projects · Udhayan

An end-to-end repository consolidating all five IoT tasks developed for the **ESP32-C3** RISC-V and **ESP32** platforms, featuring **bare-metal Embedded Rust (`no_std`)**, asynchronous **Embassy**, **MQTT**, **Google Assistant** voice integration, and **Firebase** real-time telemetry.

🌐 **Live Documentation & Video Portfolio:**  
👉 [**https://udhayan27-dev.github.io/IoT-portfolio/**](https://udhayan27-dev.github.io/IoT-portfolio/)

---

## Repository Structure

| Directory | Task | Platform & Language | Key Features |
| :--- | :--- | :--- | :--- |
| [**`task1-web-control/`**](task1-web-control/) | **Task 1** | ESP32-C3 · Embedded Rust | Async HTTP web server on port 80 toggling an LED via `embassy-net` and `esp-radio`. |
| [**`task2-adafruit-mqtt/`**](task2-adafruit-mqtt/) | **Task 2** | ESP32-C3 · Embedded Rust | Zero-allocation async MQTT 3.1.1 client controlling a 230V AC relay via Adafruit IO. |
| [**`task3-voice-automation/`**](task3-voice-automation/) | **Task 3** | ESP32-C3 · Embedded Rust | Google Assistant voice control integrated via IFTTT webhooks and Adafruit IO MQTT. |
| [**`task4-firebase-dashboard/`**](task4-firebase-dashboard/) | **Task 4** | ESP32 · C++ & Web | DHT11 & LDR telemetry streaming to Firebase Realtime Database with Auth login. |
| [**`task5-logging-automation/`**](task5-logging-automation/) | **Task 5** | ESP32 · C++ & Web | Ambient light threshold automation, timestamped `/logs`, history charts & CSV export. |

---

## Hardware Rig Pinout

| Peripheral | ESP32-C3 / ESP32 Pin | Role |
| :--- | :--- | :--- |
| **LED + 220 Ω Resistor** | `GPIO 10` | Digital Output (Task 1) |
| **Relay Module (AC Bulb)** | `GPIO 10` (Rust) / `GPIO 5` (Firebase) | Optocoupled Active-LOW AC Mains Relay (Tasks 2–5) |
| **DHT11 Sensor** | `GPIO 4` | Temperature & Relative Humidity (Tasks 4–5) |
| **LDR Light Sensor Divider** | `GPIO 34` | Ambient Illuminance ADC Input (Tasks 4–5) |

---

## Toolchain & Setup

### Embedded Rust Projects (Tasks 1, 2, 3)

The Rust firmware targets the 32-bit RISC-V architecture:
```bash
# Add RISC-V bare-metal target and standard library source
rustup target add riscv32imc-unknown-none-elf
rustup component add rust-src

# Install Espressif serial flash runner
cargo install espflash --locked
```

To build and flash any Rust task:
```bash
cd task1-web-control   # or task2-adafruit-mqtt / task3-voice-automation
cargo run --release
```

### Firebase Projects (Tasks 4 & 5)

Contains firmware for Arduino / ESP-IDF and web dashboards under `dashboard/public`.
```bash
cd task4-firebase-dashboard/dashboard
# Deploy to Firebase Hosting (optional)
firebase deploy --only hosting
```

---

## Author & Links

- **Author:** Udhayan ([@Udhayan27-Dev](https://github.com/Udhayan27-Dev))
- **Live Portfolio:** [https://udhayan27-dev.github.io/IoT-portfolio/](https://udhayan27-dev.github.io/IoT-portfolio/)
- **Portfolio Repo:** [https://github.com/Udhayan27-Dev/IoT-portfolio](https://github.com/Udhayan27-Dev/IoT-portfolio)
