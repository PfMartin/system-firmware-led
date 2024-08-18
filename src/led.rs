use anyhow::Result;
use smart_leds::{SmartLedsWrite, RGB8};
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

pub type RgbColor = (u8, u8, u8);

pub struct Led {
    ch_num: u8,
    gpio: u32,
    num_leds: usize,
}

impl Led {
    pub fn new(ch_num: u8, gpio: u32, num_leds: usize) -> Led {
        Led {
            ch_num,
            gpio,
            num_leds,
        }
    }

    pub fn set_led_color(&self, color: RgbColor) -> Result<()> {
        let mut led_strip = Ws2812Esp32Rmt::new(self.ch_num, self.gpio)?;

        let (red, green, blue) = color;
        let led_color = RGB8::new(red, green, blue);

        let pixels = std::iter::repeat(led_color).take(self.num_leds);
        led_strip.write(pixels)?;

        Ok(())
    }
}
