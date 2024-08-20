use crate::{
    led::{IndicatorLedConfig, Led, RgbColor},
    status::Status,
};
use anyhow::{anyhow, Context, Error, Result};
use embedded_svc::mqtt::client::QoS;
use esp_idf_svc::mqtt::client::{
    EspMqttClient, EspMqttConnection,
    EventPayload::{Connected, Disconnected, Received},
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
    stop_signal: Arc<Mutex<bool>>,
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
            stop_signal: Arc::new(Mutex::new(false)),
        }
    }

    pub fn start_listening_loop(
        self: Arc<Self>,
        mut connection: EspMqttConnection,
    ) -> Result<(), Error> {
        info!("MQTT Listening for messages");

        let mut thread_handles = vec![];

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
                                "Received message from topic {:?}, details: {t}, id: {id}",
                                details
                            );

                            let color_data: Result<ColorData, serde_json::Error> =
                                serde_json::from_slice(data);
                            if let Ok(color) = color_data {
                                let rgb_color: RgbColor = (color.red, color.green, color.blue);

                                let mut locked_status = self.status_mutex.lock().unwrap();
                                locked_status.set_new_status(rgb_color)?;
                                drop(locked_status);

                                self.led_strip
                                    .set_led_color(rgb_color)
                                    .with_context(|| "Failed to set led color")?;
                            }
                        }
                    }
                }
                Connected(_) => {
                    info!("Connected to message broker");
                    *self.stop_signal.lock().unwrap() = false;

                    self.indicator_led
                        .set_led_color(self.indicator_led_config.message_broker_connection)?;

                    let publisher = self.clone();
                    thread_handles.push(publisher.start_publish_loop());

                    let subscriber = self.clone();
                    thread_handles.push(subscriber.subscribe());
                }
                Disconnected => {
                    info!("Disconnected from message broker");
                    *self.stop_signal.lock().unwrap() = true;

                    self.indicator_led
                        .set_led_color(self.indicator_led_config.wifi_connection)?;
                }
                _ => info!("[Queue] Event: {}", event.payload()),
            }
        }

        for handle in thread_handles {
            info!("Joining handle");
            let _ = handle
                .join()
                .map_err(|e| anyhow!("Thread panicked: {:?}", e))?;
        }

        info!("Connection closed");

        Ok(())
    }

    pub fn start_publish_loop(self: Arc<Self>) -> JoinHandle<Result<(), Error>> {
        info!("Starting to publish the status");

        thread::spawn(move || -> Result<()> {
            let stop_signal = self.stop_signal.clone();

            loop {
                let thread_stopper = stop_signal.lock().unwrap();
                if *thread_stopper {
                    info!("Stopping publish loop");
                    return Ok(());
                }
                drop(thread_stopper);

                let mut locked_client = self.client_mutex.lock().unwrap();
                let locked_status = self.status_mutex.lock().unwrap();

                locked_client.enqueue(
                    self.publish_topic,
                    QoS::AtLeastOnce,
                    false,
                    &locked_status.to_message()?,
                )?;
                drop(locked_client);
                drop(locked_status);

                sleep(Duration::from_secs(self.publish_status_interval_s.into()));
            }
        })
    }

    pub fn subscribe(self: Arc<Self>) -> JoinHandle<Result<(), Error>> {
        thread::spawn(move || -> Result<()> {
            info!("Trying to subscribe to topic: '{}'", self.subscribe_topic);

            let stop_signal = self.stop_signal.clone();

            loop {
                let thread_stopper = stop_signal.lock().unwrap();
                if *thread_stopper {
                    info!("Stopping subscribe loop");
                    return Ok(());
                }
                drop(thread_stopper);

                let mut locked_client = self.client_mutex.lock().unwrap();
                let mut locked_status = self.status_mutex.lock().unwrap();
                if let Err(e) = locked_client.subscribe(self.subscribe_topic, QoS::AtMostOnce) {
                    error!(
                        "Failed to subscribe to topic \"{}\": {}, retrying...",
                        &self.subscribe_topic, e
                    );

                    sleep(Duration::from_millis(500));

                    continue;
                }
                drop(locked_client);

                info!("Subscribed to topic: \"{}\"", &self.subscribe_topic);
                locked_status.set_is_subscribed(true)?;
                drop(locked_status);

                break;
            }

            Ok(())
        })
    }
}
