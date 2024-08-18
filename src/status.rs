use crate::led_control::RgbColor;
use anyhow::{Error, Result};
use embedded_svc::mqtt::client::QoS;
use esp_idf_svc::mqtt::client::EspMqttClient;
use serde::{Deserialize, Serialize};
use serde_json::to_string;
use std::{
    sync::{Arc, Mutex},
    thread::{self, sleep, JoinHandle},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Status {
    client_name: &'static str,
    num_strip_leds: usize,
    pub last_changed: u64,
    pub current_color: RgbColor,
    pub last_color: RgbColor,
}

impl Status {
    pub fn new(client_name: &'static str, num_strip_leds: usize) -> Status {
        Status {
            client_name,
            num_strip_leds,
            last_changed: 0,
            current_color: (0, 0, 0),
            last_color: (0, 0, 0),
        }
    }

    pub fn set_new_status(&mut self, new_color: RgbColor) -> Result<()> {
        let now = SystemTime::now();
        let duration_since_epoch = now.duration_since(UNIX_EPOCH)?;

        self.last_changed = duration_since_epoch.as_secs();
        self.last_color = self.current_color;
        self.current_color = new_color;

        Ok(())
    }

    pub fn to_message(self) -> Result<Vec<u8>> {
        Ok(to_string(&self)?.into_bytes())
    }

    pub fn publish_loop(
        &self,
        client_mutex: &Arc<Mutex<EspMqttClient<'static>>>,
        status_mutex: &Arc<Mutex<Status>>,
        publish_topic: &'static str,
    ) -> JoinHandle<Result<(), Error>> {
        let publish_client_mutex = Arc::clone(client_mutex);
        let publish_status_mutex = Arc::clone(status_mutex);

        thread::spawn(move || -> Result<()> {
            loop {
                sleep(Duration::from_millis(2000));

                let mut locked_client = publish_client_mutex.lock().unwrap();
                let locked_status_mutex = publish_status_mutex.lock().unwrap();

                locked_client.enqueue(
                    publish_topic,
                    QoS::AtLeastOnce,
                    false,
                    &locked_status_mutex.to_message()?,
                )?;
            }
        })
    }
}
