use anyhow::{anyhow, Context, Result};
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use heapless::String;
use led::{set_led_color, RgbColor};
use log::info;
use paho_mqtt as mqtt;
use rand::Rng;
use status::Status;
use std::{
    sync::{Arc, Mutex},
    thread,
};
use std::{thread::sleep, time::Duration};
use wifi::connect_to_wifi;

mod led;
mod status;
mod wifi;

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
    #[default("tcp://localhost:1883")]
    mqtt_broker_address: &'static str,
    #[default("status/led-office")]
    mqtt_publish_topic: &'static str,
    #[default("client-1")]
    mqtt_client_id: &'static str,
}

const LED_STRIP_INITIAL_COLOR: RgbColor = (255, 150, 50);
const INDICATOR_LED_INITIAL_COLOR: RgbColor = (0, 20, 20);

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let app_config = CONFIG;

    let _ = set_led_color(LED_STRIP_INITIAL_COLOR, 1, app_config.led_strip_gpio)?;
    let _ = set_led_color(
        INDICATOR_LED_INITIAL_COLOR,
        0,
        app_config.indicator_led_gpio,
    )?;

    let peripherals = Peripherals::take()?;
    let sysloop = EspSystemEventLoop::take()?;

    let wifi_ssid =
        String::<32>::try_from(app_config.wifi_ssid).map_err(|e| anyhow!("Error: {:?}", e))?;
    let wifi_psk =
        String::<64>::try_from(app_config.wifi_psk).map_err(|e| anyhow!("Error: {:?}", e))?;

    let _wifi_connection = connect_to_wifi(wifi_ssid, wifi_psk, peripherals.modem, sysloop)?;

    let mut client = mqtt::Client::new(app_config.mqtt_broker_address)?;
    let conn_opts = mqtt::ConnectOptionsBuilder::new()
        .client_id(app_config.mqtt_client_id)
        .keep_alive_interval(Duration::new(30, 0))
        .finalize();

    client.connect(conn_opts)?;
    let client_mutex = Arc::new(Mutex::new(client));

    let status = Status::new();
    let status_mutex = Arc::new(Mutex::new(status));

    let mut thread_handles = vec![];

    // let publish_client_mutex = Arc::clone(&client_mutex);
    let publish_status_mutex = Arc::clone(&status_mutex);
    thread_handles.push(thread::spawn(move || -> Result<()> {
        loop {
            sleep(Duration::from_millis(2000));
            let s = publish_status_mutex.lock().unwrap();
            info!(
                "Publishing current status: last_changed: {:?}, current_color: {:?}, last_color: {:?}",
                s.last_changed, s.current_color, s.last_color
            );

            let c = publish_client_mutex.lock().unwrap();
            let msg = mqtt::Message::new(app_config.mqtt_publish_topic, "Hello World", mqtt::QOS_0);
            c.publish(msg)?;
        }
    }));

    let subscription_status_mutex = Arc::clone(&status_mutex);
    thread_handles.push(thread::spawn(move || loop {
        sleep(Duration::from_millis(2000));
        // info!("Waiting for messages...");
        let mut rng = rand::thread_rng();
        let mut s = subscription_status_mutex.lock().unwrap();

        let new_color = (rng.gen(), rng.gen(), rng.gen());

        s.set_new_status(new_color)?;
        let _ = set_led_color(new_color, 0, app_config.indicator_led_gpio)
            .with_context(|| format!("Failed to set led color"))?;
    }));

    for handle in thread_handles {
        let _ = handle
            .join()
            .map_err(|e| anyhow!("Thread panicked: {:?}", e))?;
    }

    client.disconnect(None)?;

    Ok(())
}
