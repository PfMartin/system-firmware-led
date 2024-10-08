use anyhow::Result;
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;

use led::{IndicatorLedConfig, Led, RgbColor};
use message_controller::MessageController;
use mqtt_client::MqttClient;
use status::Status;
use std::sync::Arc;
use wifi_control::connect_to_wifi;

mod led;
mod message_controller;
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
    #[default(30)]
    publish_status_interval_s: u32,
    #[default("mqtt://localhost:1883")]
    mqtt_broker_address: &'static str,
    #[default("led-color/office")]
    mqtt_subscribe_topic: &'static str,
    #[default("status/led-office")]
    mqtt_publish_topic: &'static str,
    #[default("client-1")]
    mqtt_client_id: &'static str,
    #[default(1)]
    num_leds: usize,
}

const LED_STRIP_INITIAL_COLOR: RgbColor = (255, 150, 50);

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let app_config = CONFIG;

    let indicator_led_config = IndicatorLedConfig::new();

    let indicator_led = Led::new(1, app_config.indicator_led_gpio, 1);
    indicator_led.set_led_color(indicator_led_config.disconnected)?;

    let led_strip = Led::new(0, app_config.led_strip_gpio, app_config.num_leds);
    led_strip.set_led_color(LED_STRIP_INITIAL_COLOR)?;

    let peripherals = Peripherals::take()?;
    let sysloop = EspSystemEventLoop::take()?;

    let wifi_connection = connect_to_wifi(
        app_config.wifi_ssid,
        app_config.wifi_psk,
        peripherals.modem,
        sysloop,
    )?;

    if wifi_connection.is_connected()? {
        indicator_led.set_led_color(indicator_led_config.wifi_connection)?;
    }

    let client = MqttClient::new(app_config.mqtt_broker_address, app_config.mqtt_client_id)?;

    let status = Status::new(
        app_config.mqtt_client_id,
        app_config.num_leds,
        app_config.mqtt_subscribe_topic,
    );

    let message_controller = MessageController::new(
        client.client,
        status,
        app_config.publish_status_interval_s,
        app_config.mqtt_publish_topic,
        app_config.mqtt_subscribe_topic,
        indicator_led,
        led_strip,
    );

    let controller_arc = Arc::new(message_controller);
    controller_arc.start_listening_loop(client.connection)?;

    Ok(())
}
