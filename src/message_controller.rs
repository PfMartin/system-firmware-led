use crate::{led::Led, status::Status};
use anyhow::{Error, Result};
use embedded_svc::mqtt::client::QoS;
use esp_idf_svc::mqtt::client::EspMqttClient;
use log::{error, info};
use std::{
    sync::{Arc, Mutex},
    thread::{self, sleep, JoinHandle},
    time::Duration,
};

pub struct MessageController {
    client_mutex: Arc<Mutex<EspMqttClient<'static>>>,
    status_mutex: Arc<Mutex<Status>>,
    publish_topic: &'static str,
    subscribe_topic: &'static str,
    controlled_led: Led,
}

impl MessageController {
    pub fn new(
        client: EspMqttClient<'static>,
        status: Status,
        publish_topic: &'static str,
        subscribe_topic: &'static str,
        controlled_led: Led,
    ) -> MessageController {
        let client_mutex = Arc::new(Mutex::new(client));
        let status_mutex = Arc::new(Mutex::new(status));

        MessageController {
            client_mutex,
            status_mutex,
            publish_topic,
            subscribe_topic,
            controlled_led,
        }
    }

    pub fn start_publish_loop(self: Arc<Self>) -> JoinHandle<Result<(), Error>> {
        thread::spawn(move || -> Result<()> {
            loop {
                sleep(Duration::from_millis(2000));

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
                if let Err(e) =
                    locked_client_mutex.subscribe(&self.subscribe_topic, QoS::AtMostOnce)
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
