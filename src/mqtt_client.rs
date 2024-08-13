use anyhow::Result;
// use embedded_svc::mqtt::client::QoS;
use esp_idf_svc::mqtt::client::{EspMqttClient, EspMqttConnection, MqttClientConfiguration};
pub struct MqttClient {
    pub client: EspMqttClient<'static>,
    pub connection: EspMqttConnection,
}

impl MqttClient {
    pub fn new(broker_address: &'static str, client_id: &'static str) -> Result<MqttClient> {
        let (client, connection) = EspMqttClient::new(
            broker_address,
            &MqttClientConfiguration {
                client_id: Some(client_id),
                ..Default::default()
            },
        )?;

        Ok(MqttClient { client, connection })
    }

    // pub fn publish_message(&mut self, topic: &str, payload: &[u8]) -> Result<()> {
    //     self.client
    //         .enqueue(topic, QoS::AtLeastOnce, false, payload)?;

    //     Ok(())
    // }
}
