#![no_std]
#![no_main]

use ch32_hal as hal;
use core::iter::once;
use hal::delay::Delay;
use hal::gpio::{Level, Output, Speed};
use smart_leds::hsv::{hsv2rgb, Hsv};
use smart_leds::SmartLedsWrite;
use ws2812_delay::Ws2812;

#[qingke_rt::entry]
fn main() -> ! {
    let mut config = hal::Config::default();
    config.rcc = hal::rcc::Config::SYSCLK_FREQ_144MHZ_HSI;
    let p = hal::init(config);

    let mut delay = Delay;

    let pin = Output::new(p.PA4, Level::Low, Speed::High);
    let mut led = Ws2812::new(Delay, pin);

    let mut color = Hsv {
        hue: 40,
        sat: 255,
        val: 50,
    };

    loop {
        color.hue = color.hue.wrapping_add(1);
        let _ = led.write(once(hsv2rgb(color)));
        delay.delay_ms(1);
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}