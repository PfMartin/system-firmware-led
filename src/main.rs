use anyhow::{anyhow, Result};
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use heapless::String;
use led_strip::set_led_color;
use log::info;
use std::{thread::sleep, time::Duration};
use wifi::connect_to_wifi;

mod led_strip;
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
}

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let app_config = CONFIG;

    let _ = set_led_color(255, 150, 50, 1, app_config.led_strip_gpio)?;
    let _ = set_led_color(0, 0, 20, 0, app_config.indicator_led_gpio)?;

    let peripherals = Peripherals::take()?;
    let sysloop = EspSystemEventLoop::take()?;

    let wifi_ssid = match String::<32>::try_from(app_config.wifi_ssid) {
        Ok(v) => v,
        Err(e) => return Err(anyhow!("Error: {:?}", e)),
    };

    let wifi_psk = match String::<64>::try_from(app_config.wifi_psk) {
        Ok(v) => v,
        Err(e) => return Err(anyhow!("Error: {:?}", e)),
    };

    let _wifi_connection = connect_to_wifi(wifi_ssid, wifi_psk, peripherals.modem, sysloop)?;

    loop {
        info!("Waiting for messages...");
        sleep(Duration::from_millis(1000))
    }
}
