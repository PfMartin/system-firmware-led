use anyhow::Result;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::led_control::RgbColor;

pub struct Status {
    pub last_changed: u64,
    pub current_color: RgbColor,
    pub last_color: RgbColor,
}

impl Status {
    pub fn new() -> Status {
        return Status {
            last_changed: 0,
            current_color: (0, 0, 0),
            last_color: (0, 0, 0),
        };
    }

    pub fn set_new_status(&mut self, new_color: RgbColor) -> Result<()> {
        let now = SystemTime::now();
        let duration_since_epoch = now.duration_since(UNIX_EPOCH)?;

        self.last_changed = duration_since_epoch.as_secs();
        self.last_color = self.current_color;
        self.current_color = new_color;

        Ok(())
    }
}
