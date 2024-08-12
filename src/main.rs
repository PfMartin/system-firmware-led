use anyhow::{anyhow, Context, Result};
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use heapless::String;
use led_strip::set_led_color;
use log::info;
use rand::Rng;
use status_store::Status;
use std::{
    sync::{Arc, Mutex},
    thread,
};
use std::{thread::sleep, time::Duration};
use wifi::connect_to_wifi;

mod led_strip;
mod status_store;
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
    let _ = set_led_color(0, 20, 20, 0, app_config.indicator_led_gpio)?;

    let peripherals = Peripherals::take()?;
    let sysloop = EspSystemEventLoop::take()?;

    let wifi_ssid =
        String::<32>::try_from(app_config.wifi_ssid).map_err(|e| anyhow!("Error: {:?}", e))?;
    let wifi_psk =
        String::<64>::try_from(app_config.wifi_psk).map_err(|e| anyhow!("Error: {:?}", e))?;

    let _wifi_connection = connect_to_wifi(wifi_ssid, wifi_psk, peripherals.modem, sysloop)?;

    let status = Status::new();
    let status_mutex = Arc::new(Mutex::new(status));

    let mut thread_handles = vec![];

    let mutex_clone = Arc::clone(&status_mutex);
    thread_handles.push(thread::spawn(move || -> Result<()> {
        loop {
            sleep(Duration::from_millis(2000));
            let s = mutex_clone.lock().unwrap();
            info!(
                "Current status: last_changed: {:?}, current_color: {:?}, last_color: {:?}",
                s.last_changed, s.current_color, s.last_color
            );
        }
    }));

    let subscription_mutex = Arc::clone(&status_mutex);
    thread_handles.push(thread::spawn(move || loop {
        sleep(Duration::from_millis(2000));
        info!("Waiting for messages...");
        let mut rng = rand::thread_rng();
        let mut s = subscription_mutex.lock().unwrap();

        let new_color = (rng.gen(), rng.gen(), rng.gen());
        info!("Setting new color: {:?}", new_color);

        s.set_new_status(new_color);
        let _ = set_led_color(
            new_color.0,
            new_color.1,
            new_color.2,
            0,
            app_config.indicator_led_gpio,
        )
        .with_context(|| format!("Failed to set led color"))?;
    }));

    for handle in thread_handles {
        let _ = handle
            .join()
            .map_err(|e| anyhow!("Thread panicked: {:?}", e))?;
    }

    Ok(())
}
