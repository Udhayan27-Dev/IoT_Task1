#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

extern crate alloc;

use alloc::format;
use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_time::{Duration, Timer};
use embedded_io_async::Write as _;
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::ram;
use esp_hal::timer::timg::TimerGroup;
use log::info;

use alloc::string::ToString;

esp_bootloader_esp_idf::esp_app_desc!();

const SSID: &str = "YOUR_WIFI_SSID";
const PASSWORD: &str = "YOUR_WIFI_PASSWORD";

macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[ram(reclaimed)] size: 64 * 1024);
    esp_alloc::heap_allocator!(size: 36 * 1024);
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    info!("Embassy initialized!");

    let led = Output::new(peripherals.GPIO10, Level::Low, OutputConfig::default());

    let (controller, interfaces) =
        esp_radio::wifi::new(peripherals.WIFI, Default::default()).unwrap();
    let wifi_interface = interfaces.station;

    let net_config = embassy_net::Config::dhcpv4(Default::default());
    let rng = esp_hal::rng::Rng::new();
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    let (stack, runner) = embassy_net::new(
        wifi_interface,
        net_config,
        mk_static!(
            embassy_net::StackResources<3>,
            embassy_net::StackResources::<3>::new()
        ),
        seed,
    );

    spawner.spawn(connection(controller).expect("msg"));
    spawner.spawn(net_task(runner).expect("msg"));

    info!("Waiting for IP address...");
    loop {
        if let Some(cfg) = stack.config_v4() {
            info!("Got IP: {}", cfg.address);
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    spawner.spawn(web_task(stack, led).expect("failed to spawn web task"));

    loop {
        Timer::after(Duration::from_secs(3600)).await;
    }
}   

#[embassy_executor::task]
async fn connection(mut controller: esp_radio::wifi::WifiController<'static>) {
    info!("start connection task");

    let station_config = esp_radio::wifi::Config::Station(
        esp_radio::wifi::sta::StationConfig::default()
            .with_ssid(SSID)
            .with_password(PASSWORD.to_string()),
    );
    controller.set_config(&station_config).unwrap();

    loop {
        info!("Connecting...");
        match controller.connect_async().await {
            Ok(info) => info!("Connected! {:?}", info),
            Err(e) => {
                info!("Failed to connect: {:?}", e);
                Timer::after_secs(5).await;
                continue;
            }
        }

        let _ = controller.wait_for_disconnect_async().await;
        info!("Disconnected, retrying...");
        Timer::after_secs(5).await;
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, esp_radio::wifi::Interface<'static>>) {
    runner.run().await
}

#[embassy_executor::task]
async fn web_task(stack: embassy_net::Stack<'static>, mut led: Output<'static>) {
    let mut rx_buffer = [0u8; 1024];
    let mut tx_buffer = [0u8; 1024];

    loop {
        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(10)));

        info!("Listening on port 80...");
        if let Err(e) = socket.accept(80).await {
            info!("accept error: {:?}", e);
            continue;
        }

        let mut buf = [0u8; 512];
        let n = match socket.read(&mut buf).await {
            Ok(0) => {
                socket.close();
                continue;
            }
            Ok(n) => n,
            Err(e) => {
                info!("read error: {:?}", e);
                continue;
            }
        };

        let request = core::str::from_utf8(&buf[..n]).unwrap_or("");
        let path = request.split_whitespace().nth(1).unwrap_or("/");
        info!("Request path: {}", path);

        let body = if path == "/on" {
            led.set_high();
            "<html><body><h1>LED is ON</h1><a href=\"/off\">Turn OFF</a></body></html>"
        } else if path == "/off" {
            led.set_low();
            "<html><body><h1>LED is OFF</h1><a href=\"/on\">Turn ON</a></body></html>"
        } else {
            "<html><body><h1>ESP32-C3 LED Control</h1><a href=\"/on\">ON</a> | <a href=\"/off\">OFF</a></body></html>"
        };

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );

        let _ = socket.write_all(response.as_bytes()).await;
        let _ = socket.flush().await;
        socket.close();
        Timer::after_millis(100).await;
        socket.abort();
    }
}
