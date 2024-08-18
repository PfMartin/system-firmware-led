use anyhow::{anyhow, Context, Error, Result};
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use led_control::{set_led_color, RgbColor};
use log::info;
use mqtt_client::MqttClient;
use rand::Rng;
use status::Status;
use std::{
    sync::{Arc, Mutex},
    thread::{self, sleep, JoinHandle},
    time::Duration,
};
use wifi_control::connect_to_wifi;

mod led_control;
mod mqtt_client;
mod status;
mod wifi_control;

#[toml_cfg::toml_config]
pub struct Config {
    #[default("")]
    wifi_ssid: &'static str,
    #[default("")]
    wifi_psk: &'static str,
    #[default(6)]
    led_strip_gpio: u32,
    #[default(8)]
    indicator_led_gpio: u32,
    #[default("mqtt://localhost:1883")]
    mqtt_broker_address: &'static str,
    #[default("status/led-office")]
    mqtt_publish_topic: &'static str,
    #[default("client-1")]
    mqtt_client_id: &'static str,
    #[default(1)]
    num_leds: usize,
}

const LED_STRIP_INITIAL_COLOR: RgbColor = (255, 150, 50);
const INDICATOR_LED_INITIAL_COLOR: RgbColor = (0, 20, 20);

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let app_config = CONFIG;

    initialize_leds(
        app_config.led_strip_gpio,
        app_config.indicator_led_gpio,
        app_config.num_leds,
    )?;

    let peripherals = Peripherals::take()?;
    let sysloop = EspSystemEventLoop::take()?;

    let _wifi_connection = connect_to_wifi(
        app_config.wifi_ssid,
        app_config.wifi_psk,
        peripherals.modem,
        sysloop,
    )?;

    let mut client = MqttClient::new(app_config.mqtt_broker_address, app_config.mqtt_client_id)?;

    let mut thread_handles = vec![];
    thread_handles.push(thread::spawn(move || -> Result<()> {
        info!("MQTT Listening for messages");

        while let Ok(event) = client.connection.next() {
            info!("[Queue] Event: {}", event.payload())
        }

        info!("Connection closed");

        Ok(())
    }));

    let client_mutex = Arc::new(Mutex::new(client.client));

    let status = Status::new(app_config.mqtt_client_id, app_config.num_leds);
    let status_mutex = Arc::new(Mutex::new(status));

    thread_handles.push(status.publish_loop(
        &client_mutex,
        &status_mutex,
        app_config.mqtt_publish_topic,
    ));
    thread_handles.push(subscribe_color_change_loop(
        &status_mutex,
        app_config.indicator_led_gpio,
    ));

    for handle in thread_handles {
        let _ = handle
            .join()
            .map_err(|e| anyhow!("Thread panicked: {:?}", e))?;
    }

    Ok(())
}

fn initialize_leds(
    led_strip_gpio: u32,
    inidicator_led_gpio: u32,
    num_strip_leds: usize,
) -> Result<()> {
    set_led_color(LED_STRIP_INITIAL_COLOR, 1, led_strip_gpio, num_strip_leds)?;
    set_led_color(INDICATOR_LED_INITIAL_COLOR, 0, inidicator_led_gpio, 1)?;

    Ok(())
}

fn subscribe_color_change_loop(
    status_mutex: &Arc<Mutex<Status>>,
    indicator_led_gpio: u32,
) -> JoinHandle<Result<(), Error>> {
    let subscription_status_mutex = Arc::clone(status_mutex);

    thread::spawn(move || loop {
        sleep(Duration::from_millis(2000));
        let mut rng = rand::thread_rng();
        let mut s = subscription_status_mutex.lock().unwrap();

        let new_color = (rng.gen(), rng.gen(), rng.gen());

        s.set_new_status(new_color)?;
        set_led_color(new_color, 0, indicator_led_gpio, 1)
            .with_context(|| "Failed to set led color")?;
    })
}
