use anyhow::Result;
use smart_leds::{SmartLedsWrite, RGB8};
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

pub type RgbColor = (u8, u8, u8);

pub fn set_led_color(color: RgbColor, ch_num: u8, gpio: u32, num_leds: usize) -> Result<()> {
    let mut led_strip = Ws2812Esp32Rmt::new(ch_num, gpio)?;

    let (red, green, blue) = color;
    let led_color = RGB8::new(red, green, blue);

    let pixels = std::iter::repeat(led_color).take(num_leds);
    led_strip.write(pixels)?;

    Ok(())
}
