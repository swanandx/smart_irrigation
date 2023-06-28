use std::thread;
use std::time::Duration;

use esp_idf_hal::gpio::*;
use esp_idf_hal::peripherals::Peripherals;

use anyhow::*;
use log::*;

use esp_idf_svc::sntp;
use esp_idf_svc::sntp::SyncStatus;
use esp_idf_svc::systime::EspSystemTime;

// Wi-Fi
use embedded_svc::wifi::*;
use esp_idf_svc::eventloop::*;
use esp_idf_svc::netif::*;
use esp_idf_svc::wifi::{EspWifi};

use esp_idf_svc::nvs::EspDefaultNvsPartition;

use esp_idf_svc::{
    log::EspLogger,
    mqtt::client::{EspMqttClient, MqttClientConfiguration},
};
use embedded_svc::mqtt::client::Event::Received;
use embedded_svc::mqtt::client::{Connection, QoS};
use embedded_svc::mqtt::client::MessageImpl;
use embedded_svc::mqtt::client::Message;

use std::result::Result::Ok;

const WIFI_SSID: &str = "Wokwi-GUEST";
const WIFI_PASS: &str = "";
const LED_TOPIC: &str = "edi_15"

fn main() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;
    let nvs_default_partition = EspDefaultNvsPartition::take()?;
    let mut led = PinDriver::output(peripherals.pins.gpio4)?;

    let mut wifi = EspWifi::new(
        peripherals.modem,
        sysloop.clone(),
        Some(nvs_default_partition.clone()),
    )?;

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: "Wokwi-GUEST".into(),
        password: "".into(),
        auth_method: AuthMethod::None,
        ..Default::default()
    }))?;

    wifi.start()?;
    wifi.connect()?;

    let sntp = sntp::EspSntp::new_default()?;
    info!("SNTP initialized, waiting for status!");

    while sntp.get_sync_status() != SyncStatus::Completed {}

    info!("SNTP status received!");

    let conf = MqttClientConfiguration::default();

    let (mut client, mut connection) =
        EspMqttClient::new_with_conn("mqtt://mqtt-dashboard.com:1883", &conf)?;

    info!("MQTT client started");
    thread::spawn(move || {
        info!("MQTT Listening for messages");

        while let Some(msg) = connection.next() {
            match msg {
                Err(e) => info!("MQTT Message ERROR: {}", e),
                Ok(Received(msg)) => {
                    process_message(msg, &mut led);
                },
                Ok(m) => { info!("MQTT Message: {:?}", m) }
            }
        }

        info!("MQTT connection loop exit");
    });

    client.subscribe(LED_TOPIC, QoS::AtMostOnce)?;

    loop {
        thread::sleep(Duration::from_millis(1000));
    }
}


fn process_message(message: MessageImpl, led: &mut PinDriver<Gpio4, esp_idf_hal::gpio::Output>) {
    match message.details() {
        esp_idf_svc::mqtt::client::Details::Complete => {
            info!("{:?}", message);
            let message_data: &[u8] = message.data();
            if let Ok(s) = std::str::from_utf8(message_data) {
                info!("{}", s);
                match s {
                    "on" | "ON" =>  { led.set_high(); },
                    "off" | "OFF" => { led.set_low(); },
                    e => error!("Invalid command {e}")
                };
            }
        }
        _ => error!("Could not set board LED"),
    }
}

/*
[package]
name = "rust-project-esp32"
version = "0.1.0"
authors = ["Sergio Gasquez <sergio.gasquez@gmail.com>"]
edition = "2021"
resolver = "2"

[profile.release]
opt-level = "s"

[profile.dev]
debug = true    # Symbols are nice and they don't increase the size on Flash
opt-level = "z"

[features]
pio = ["esp-idf-sys/pio"]

[dependencies]
esp-idf-sys = { version = "0.33.0", features = ["binstart"] }
esp-idf-hal = "0.41.1"
esp-idf-svc = "0.46.0"
embedded-svc = "0.25.1"
embedded-hal = "0.2.7"
anyhow = "1.0.71"
log = "0.4"


[build-dependencies]
embuild = "0.31.1"

*/
