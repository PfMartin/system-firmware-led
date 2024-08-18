use crate::led::RgbColor;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::to_string;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Status {
    pub client_name: &'static str,
    pub num_strip_leds: usize,
    pub change_color_topic: &'static str,
    pub last_changed: u64,
    pub current_color: RgbColor,
    pub last_color: RgbColor,
}

impl Status {
    pub fn new(
        client_name: &'static str,
        num_strip_leds: usize,
        change_color_topic: &'static str,
    ) -> Status {
        Status {
            client_name,
            num_strip_leds,
            change_color_topic,
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
}
