use crate::{led::Led, status::Status};
use anyhow::{Context, Error, Result};
use embedded_svc::mqtt::client::QoS;
use esp_idf_svc::mqtt::client::EspMqttClient;
use log::info;
use rand::Rng;
use std::{
    sync::{Arc, Mutex},
    thread::{self, sleep, JoinHandle},
    time::Duration,
};

#[derive(Clone)]
pub struct MessageController {
    client_mutex: Arc<Mutex<EspMqttClient<'static>>>,
    status_mutex: Arc<Mutex<Status>>,
    publish_topic: &'static str,
    subscribe_topic: &'static str,
}

impl MessageController {
    pub fn new(
        client: EspMqttClient<'static>,
        status: Status,
        publish_topic: &'static str,
        subscribe_topic: &'static str,
    ) -> MessageController {
        let client_mutex = Arc::new(Mutex::new(client));
        let status_mutex = Arc::new(Mutex::new(status));

        MessageController {
            client_mutex,
            status_mutex,
            publish_topic,
            subscribe_topic,
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

    pub fn start_subscribe_loop(self: Arc<Self>, led: Led) -> JoinHandle<Result<(), Error>> {
        thread::spawn(move || loop {
            sleep(Duration::from_millis(2000));
            info!("{:?}", self.subscribe_topic);
            let mut rng = rand::thread_rng();
            let mut locked_status_mutex = self.status_mutex.lock().unwrap();

            let new_color = (rng.gen(), rng.gen(), rng.gen());

            locked_status_mutex.set_new_status(new_color)?;
            led.set_led_color(new_color)
                .with_context(|| "Failed to set led color")?;
        })
    }
}
