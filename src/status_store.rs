type RgbColor = (u8, u8, u8);

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

    pub fn set_new_status(&mut self, new_color: RgbColor) {
        self.last_color = self.current_color;
        self.current_color = new_color;
    }
}
