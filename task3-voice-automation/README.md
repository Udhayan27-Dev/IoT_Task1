# Task 3: Voice-Controlled IoT Relay via Google Assistant, IFTTT & Adafruit IO

A bare-metal asynchronous embedded Rust application for the **ESP32-C3** that turns a lamp or appliance on and off via **voice commands** issued to Google Assistant (or Alexa).

By integrating **Google Assistant**, **IFTTT (If This Then That)**, and **Adafruit IO MQTT**, spoken phrases like *"Hey Google, turn on the bulb"* immediately toggle the physical relay wired to the ESP32-C3 with sub-second latency.

---

## Overview

- **MCU**: ESP32-C3 RISC-V Bare-metal (`riscv32imc-unknown-none-elf`)
- **Language**: Embedded Rust (`no_std`) with `esp-hal`, `esp-rtos`, and `embassy`
- **Voice Platform**: Google Assistant / Alexa via IFTTT
- **Cloud Broker**: Adafruit IO (`io.adafruit.com:1883`)
- **Protocol**: MQTT 3.1.1 asynchronous client with keep-alive PINGREQ
- **Hardware Output**: GPIO 10 wired to Active-LOW Relay Module

---

## System Architecture

```
[ User Speaks: "Hey Google, Turn ON the bulb" ]
                       │
                       ▼
             [ Google Assistant ]
                       │
                       ▼
            [ IFTTT Voice Applet ]
                       │
                       │ Webhook / API Call
                       ▼
            [ Adafruit IO Cloud Broker ]
                       │
                       │ MQTT PUBLISH: "ON" (feeds/bulb)
                       ▼
           [ ESP32-C3 Wi-Fi Station ]
                       │
                       ▼
         [ Async MQTT Task (Embedded Rust) ]
                       │
                       ▼
             [ GPIO 10 -> LOW ]
                       │
                       ▼
          [ 5V Relay Coil Energized ]
                       │
                       ▼
           [ 230V AC Mains Bulb Lights Up ]
```

---

## Hardware Setup

| ESP32-C3 Pin | Relay Module Pin | Description |
| :--- | :--- | :--- |
| `GPIO 10` | `IN` | Control Signal (LOW = Relay ON, HIGH = Relay OFF) |
| `5V / VIN` | `VCC` | Relay Coil Power Supply |
| `GND` | `GND` | Common Ground |

---

## IFTTT Applet Configuration

1. **Trigger (If This)**:
   - Service: **Google Assistant**
   - Trigger: *"Say a simple phrase"*
   - Phrase: *"Turn on the bulb"* / *"Turn on the light"*

2. **Action (Then That)**:
   - Service: **Adafruit**
   - Action: *"Send data to Adafruit IO"*
   - Feed Name: `bulb`
   - Data to save: `ON`

3. Create an identical counterpart applet for *"Turn off the bulb"* sending `OFF`.

---

## Building and Flashing

```bash
cargo run --release
```

Once flashed, speak your voice command to Google Assistant and watch the ESP32-C3 serial logs:
```text
>> MQTT Received: ON -> Bulb turned ON <<
```
