use crate::{
    led::{IndicatorLedConfig, Led, RgbColor},
    status::Status,
};
use anyhow::{Context, Error, Result};
use embedded_svc::mqtt::client::QoS;
use esp_idf_svc::mqtt::client::{
    EspMqttClient, EspMqttConnection,
    EventPayload::{Connected, Error as EventError, Received},
};
use log::{error, info};
use serde::Deserialize;
use std::{
    sync::{Arc, Mutex},
    thread::{self, sleep, JoinHandle},
    time::Duration,
};

#[derive(Deserialize)]
struct ColorData {
    red: u8,
    green: u8,
    blue: u8,
}

pub struct MessageController {
    client_mutex: Arc<Mutex<EspMqttClient<'static>>>,
    status_mutex: Arc<Mutex<Status>>,
    publish_status_interval_s: u32,
    publish_topic: &'static str,
    subscribe_topic: &'static str,
    indicator_led: Led,
    led_strip: Led,
    indicator_led_config: IndicatorLedConfig,
}

impl MessageController {
    pub fn new(
        client: EspMqttClient<'static>,
        status: Status,
        publish_status_interval_s: u32,
        publish_topic: &'static str,
        subscribe_topic: &'static str,
        indicator_led: Led,
        led_strip: Led,
    ) -> MessageController {
        let client_mutex = Arc::new(Mutex::new(client));
        let status_mutex = Arc::new(Mutex::new(status));

        MessageController {
            client_mutex,
            status_mutex,
            publish_status_interval_s,
            publish_topic,
            subscribe_topic,
            indicator_led,
            led_strip,
            indicator_led_config: IndicatorLedConfig::new(),
        }
    }

    pub fn start_listening_loop(
        self: Arc<Self>,
        mut connection: EspMqttConnection,
    ) -> JoinHandle<Result<(), Error>> {
        thread::spawn(move || -> Result<()> {
            info!("MQTT Listening for messages");

            while let Ok(event) = connection.next() {
                let payload = event.payload();

                match payload {
                    Received {
                        id,
                        topic,
                        data,
                        details,
                    } => {
                        if let Some(t) = topic {
                            if t == self.subscribe_topic {
                                info!(
                                    "Received message from topic {:?}, details: {:?}, id: {id}",
                                    topic, details
                                );

                                let color_data: ColorData = serde_json::from_slice(data)?;
                                let rgb_color: RgbColor =
                                    (color_data.red, color_data.green, color_data.blue);

                                let mut locked_status_mutex = self.status_mutex.lock().unwrap();
                                locked_status_mutex.set_new_status(rgb_color)?;

                                self.led_strip
                                    .set_led_color(rgb_color)
                                    .with_context(|| "Failed to set led color")?;
                            }
                        }
                    }
                    Connected(_) => {
                        self.indicator_led
                            .set_led_color(self.indicator_led_config.message_broker_connection)?;
                        info!("Connected to message broker");
                        let s = self.clone();
                        s.start_subscribe_loop();
                    }
                    EventError(e) => {
                        error!("Error: {e}");
                        let mut locked_client_mutex = self.client_mutex.lock().unwrap();
                        locked_client_mutex.unsubscribe(&self.subscribe_topic)?;
                        info!("Unsubscribed from topic '{}'", self.subscribe_topic);
                        self.indicator_led
                            .set_led_color(self.indicator_led_config.wifi_connection)?;
                    }
                    _ => info!("[Queue] Event: {}", event.payload()),
                }
            }

            info!("Connection closed");

            Ok(())
        })
    }

    pub fn start_publish_loop(self: Arc<Self>) -> JoinHandle<Result<(), Error>> {
        thread::spawn(move || -> Result<()> {
            loop {
                sleep(Duration::from_secs(self.publish_status_interval_s.into()));

                let mut locked_client = self.client_mutex.lock().unwrap();
                let locked_status_mutex = self.status_mutex.lock().unwrap();

                locked_client.enqueue(
                    self.publish_topic,
                    QoS::AtLeastOnce,
                    false,
                    &locked_status_mutex.to_message()?,
                )?;
            }
        })
    }

    pub fn start_subscribe_loop(self: Arc<Self>) -> JoinHandle<Result<(), Error>> {
        thread::spawn(move || -> Result<()> {
            loop {
                let mut locked_client_mutex = self.client_mutex.lock().unwrap();
                let mut locked_status_mutex = self.status_mutex.lock().unwrap();
                if let Err(e) = locked_client_mutex.subscribe(self.subscribe_topic, QoS::AtMostOnce)
                {
                    error!(
                        "Failed to subscribe to topic \"{}\": {}, retrying...",
                        &self.subscribe_topic, e
                    );

                    sleep(Duration::from_millis(500));

                    continue;
                }

                info!("Subscribed to topic: \"{}\"", &self.subscribe_topic);
                locked_status_mutex.set_is_subscribed(true)?;

                break;
            }

            Ok(())
        })
    }
}
