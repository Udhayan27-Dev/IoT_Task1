#![no_std]
#![no_main]

extern crate alloc;

use core::cell::RefCell;
use critical_section::Mutex;
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_net::{
    tcp::TcpSocket,
    Config, IpAddress, Runner, Stack, StackResources,
};
use embedded_io_async::{Read, Write};
use esp_backtrace as _;
use esp_hal::{
    gpio::{Level, Output, OutputConfig},
    interrupt::software::SoftwareInterruptControl,
    ram,
    rng::Rng,
    timer::timg::TimerGroup,
};
use esp_radio::wifi::{
    sta::StationConfig, Config as WifiConfig, WifiController,
};
use static_cell::StaticCell;

esp_bootloader_esp_idf::esp_app_desc!();

// Wi-Fi Credentials (replace with your 2.4 GHz credentials)
const WIFI_SSID: &str = "YOUR_WIFI_SSID";
const WIFI_PASS: &str = "YOUR_WIFI_PASSWORD";

// Adafruit IO Credentials
const AIO_USERNAME: &str = "YOUR_AIO_USERNAME";
const AIO_KEY: &str = "aio_YOUR_ADAFRUIT_IO_KEY_HERE";
const AIO_FEED_KEY: &str = "bulb";

// io.adafruit.com public IPv4 (checked 2026-09-11 — Adafruit's IPs can change;
// if this stops working again, re-resolve io.adafruit.com and update this)
const AIO_IP: [u8; 4] = [52, 4, 253, 177];
const AIO_PORT: u16 = 1883;

// MQTT keep-alive: must be sent to Adafruit IO more often than this,
// or the broker will drop the connection (spec: within 1.5x keep_alive).
const MQTT_KEEP_ALIVE_SECS: u16 = 60;
const MQTT_PING_INTERVAL_SECS: u64 = 45; // < 1.5 * keep_alive

static RELAY: Mutex<RefCell<Option<Output<'static>>>> = Mutex::new(RefCell::new(None));

#[embassy_executor::task]
async fn connection_task(mut controller: WifiController<'static>) {
    let station_config = StationConfig::default()
        .with_ssid(esp_radio::wifi::Ssid::try_from(WIFI_SSID).unwrap())
        .with_password(WIFI_PASS.into());

    let client_config = WifiConfig::Station(station_config);
    controller.set_config(&client_config).unwrap();

    loop {
        match controller.connect_async().await {
            Ok(_) => {
                esp_println::println!("Wi-Fi connected to AP!");
                let _ = controller.wait_for_disconnect_async().await;
                esp_println::println!("Wi-Fi disconnected. Reconnecting...");
            }
            Err(_) => {
                embassy_time::Timer::after_secs(2).await;
            }
        }
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, esp_radio::wifi::Interface<'static>>) {
    runner.run().await;
}

fn encode_remaining_length(mut len: usize, out: &mut alloc::vec::Vec<u8>) {
    loop {
        let mut encoded_byte = (len % 128) as u8;
        len /= 128;
        if len > 0 {
            encoded_byte |= 128;
        }
        out.push(encoded_byte);
        if len == 0 {
            break;
        }
    }
}

fn set_relay(on: bool) {
    critical_section::with(|cs| {
        if let Some(ref mut pin) = *RELAY.borrow_ref_mut(cs) {
            if on {
                pin.set_low(); // Relay ON (active-low module)
            } else {
                pin.set_high(); // Relay OFF
            }
        }
    });
}

#[embassy_executor::task]
async fn adafruit_task(stack: Stack<'static>) {
    let mut rx_buf = [0u8; 1024];
    let mut tx_buf = [0u8; 1024];

    loop {
        let mut socket = TcpSocket::new(stack, &mut rx_buf, &mut tx_buf);
        socket.set_timeout(Some(embassy_time::Duration::from_secs(90)));

        let endpoint = (
            IpAddress::v4(AIO_IP[0], AIO_IP[1], AIO_IP[2], AIO_IP[3]),
            AIO_PORT,
        );

        esp_println::println!("Connecting to Adafruit IO MQTT (port 1883)...");
        if socket.connect(endpoint).await.is_ok() {
            esp_println::println!("TCP Connected! Sending MQTT CONNECT...");

            // 1. Send MQTT CONNECT Packet
            let client_id = "xiao_esp32c3";
            let mut connect_pkt = alloc::vec::Vec::new();

            let proto_name = b"MQTT";
            let proto_level = 4u8; // 3.1.1
            let connect_flags = 0xC2u8; // CleanSession + Username + Password flags
            let keep_alive = MQTT_KEEP_ALIVE_SECS;

            let mut var_header_and_payload = alloc::vec::Vec::new();
            var_header_and_payload.extend_from_slice(&(proto_name.len() as u16).to_be_bytes());
            var_header_and_payload.extend_from_slice(proto_name);
            var_header_and_payload.push(proto_level);
            var_header_and_payload.push(connect_flags);
            var_header_and_payload.extend_from_slice(&keep_alive.to_be_bytes());

            var_header_and_payload.extend_from_slice(&(client_id.len() as u16).to_be_bytes());
            var_header_and_payload.extend_from_slice(client_id.as_bytes());

            var_header_and_payload.extend_from_slice(&(AIO_USERNAME.len() as u16).to_be_bytes());
            var_header_and_payload.extend_from_slice(AIO_USERNAME.as_bytes());

            var_header_and_payload.extend_from_slice(&(AIO_KEY.len() as u16).to_be_bytes());
            var_header_and_payload.extend_from_slice(AIO_KEY.as_bytes());

            connect_pkt.push(0x10); // CONNECT packet type
            encode_remaining_length(var_header_and_payload.len(), &mut connect_pkt);
            connect_pkt.extend_from_slice(&var_header_and_payload);

            if socket.write_all(&connect_pkt).await.is_err() || socket.flush().await.is_err() {
                esp_println::println!("Failed to send MQTT CONNECT");
                socket.close();
                embassy_time::Timer::after_secs(3).await;
                continue;
            }

            // Read CONNACK (expect 0x20, 0x02, 0x00, 0x00)
            let mut connack = [0u8; 4];
            if socket.read_exact(&mut connack).await.is_err() || connack[0] != 0x20 || connack[3] != 0x00 {
                esp_println::println!("MQTT Connection refused or failed: {:?}", connack);
                socket.close();
                embassy_time::Timer::after_secs(3).await;
                continue;
            }
            esp_println::println!("MQTT Connected successfully to Adafruit IO!");

            // 2. Send MQTT SUBSCRIBE to "<username>/feeds/<feed>"
            let topic = alloc::format!("{}/feeds/{}", AIO_USERNAME, AIO_FEED_KEY);
            let mut sub_payload = alloc::vec::Vec::new();
            let packet_id = 1u16;
            sub_payload.extend_from_slice(&packet_id.to_be_bytes());
            sub_payload.extend_from_slice(&(topic.len() as u16).to_be_bytes());
            sub_payload.extend_from_slice(topic.as_bytes());
            sub_payload.push(0x00); // QoS 0

            let mut sub_pkt = alloc::vec::Vec::new();
            sub_pkt.push(0x82); // SUBSCRIBE packet type
            encode_remaining_length(sub_payload.len(), &mut sub_pkt);
            sub_pkt.extend_from_slice(&sub_payload);

            if socket.write_all(&sub_pkt).await.is_ok() && socket.flush().await.is_ok() {
                esp_println::println!("Subscribed to feed '{}'! Awaiting dashboard toggle...", topic);
            } else {
                esp_println::println!("Failed to send SUBSCRIBE");
                socket.close();
                embassy_time::Timer::after_secs(3).await;
                continue;
            }

            // 3. Listen for incoming MQTT PUBLISH messages, and send PINGREQ
            //    periodically so Adafruit IO doesn't drop us for being idle.
            const PINGREQ: [u8; 2] = [0xC0, 0x00];
            let mut rx_stream = [0u8; 512];
            let mut ping_ticker =
                embassy_time::Ticker::every(embassy_time::Duration::from_secs(MQTT_PING_INTERVAL_SECS));

            'session: loop {
                match select(socket.read(&mut rx_stream), ping_ticker.next()).await {
                    Either::First(Ok(0)) => {
                        esp_println::println!("Connection closed by broker.");
                        break 'session;
                    }
                    Either::First(Ok(n)) => {
                        let msg = core::str::from_utf8(&rx_stream[..n]).unwrap_or("");
                        if msg.contains("ON") {
                            set_relay(true);
                            esp_println::println!(">> MQTT Received: ON -> Bulb turned ON <<");
                        } else if msg.contains("OFF") {
                            set_relay(false);
                            esp_println::println!(">> MQTT Received: OFF -> Bulb turned OFF <<");
                        }
                    }
                    Either::First(Err(_)) => {
                        esp_println::println!("MQTT read error/timeout. Reconnecting...");
                        break 'session;
                    }
                    Either::Second(_) => {
                        if socket.write_all(&PINGREQ).await.is_err() || socket.flush().await.is_err() {
                            esp_println::println!("PINGREQ failed. Reconnecting...");
                            break 'session;
                        }
                    }
                }
            }
            socket.close();
        } else {
            esp_println::println!("TCP connect to port 1883 failed. Retrying...");
        }

        embassy_time::Timer::after_secs(3).await;
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    // Correct heap initialization order
    esp_alloc::heap_allocator!(size: 72 * 1024);
    esp_alloc::heap_allocator!(#[ram(reclaimed)] size: 48 * 1024);

    let peripherals = esp_hal::init(esp_hal::Config::default());

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

    // Initial state: Level::High (Relay OFF for active-LOW)
    let relay_pin = Output::new(peripherals.GPIO10, Level::High, OutputConfig::default());
    critical_section::with(|cs| {
        *RELAY.borrow_ref_mut(cs) = Some(relay_pin);
    });

    let (wifi_controller, interfaces) = esp_radio::wifi::new(peripherals.WIFI, Default::default()).expect("wifi init");

    static STACK_RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    let resources = STACK_RESOURCES.init(StackResources::new());

    let rng = Rng::new();
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    static STACK: StaticCell<Stack<'static>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        interfaces.station,
        Config::dhcpv4(Default::default()),
        resources,
        seed,
    );
    let stack = STACK.init(stack);

    spawner.spawn(connection_task(wifi_controller).expect("spawn connection"));
    spawner.spawn(net_task(runner).expect("spawn net"));

    esp_println::println!("Connecting to Wi-Fi...");
    loop {
        if let Some(config) = stack.config_v4() {
            esp_println::println!("Wi-Fi Connected! IP: {}", config.address.address());
            break;
        }
        embassy_time::Timer::after_millis(500).await;
    }

    spawner.spawn(adafruit_task(*stack).expect("spawn adafruit task"));
}