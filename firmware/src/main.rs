#![no_std]
#![no_main]

use ch32_hal::peripherals;
use embassy_executor::Spawner;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_time::Timer;
use embassy_usb::UsbDevice;
use hal::usbd::{Driver };
use hal::{bind_interrupts};
use {ch32_hal as hal, panic_halt as _};
use static_cell::StaticCell;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::pipe::Pipe;

use core::iter::once;
use hal::delay::Delay;
use hal::gpio::{Level, Output, Speed};
use smart_leds::hsv::{hsv2rgb, Hsv};
use smart_leds::SmartLedsWrite;
use ws2812_delay::Ws2812;

bind_interrupts!(struct Irqs {
    USB_LP_CAN1_RX0 => hal::usbd::InterruptHandler<hal::peripherals::USBD>;
});

pub static LOGGING_PIPE: Pipe<CriticalSectionRawMutex, 64> = Pipe::new();

#[embassy_executor::task]
async fn cdc_task(
    mut cdc: CdcAcmClass<'static, Driver<'static, peripherals::USBD>>
) {
    loop {
        cdc.wait_connection().await;

        loop {

            if (!LOGGING_PIPE.is_empty()) {
                let mut buf: [u8; 64] = [0; 64];

                let read = LOGGING_PIPE.read(&mut buf).await;

                if cdc.write_packet(&buf[..read]).await.is_err() {
                    break;
                }

                LOGGING_PIPE.clear();
            }

            

            Timer::after_millis(50).await;
        }
    }
}


#[embassy_executor::task]
async fn usb_task(mut usb: UsbDevice<'static, Driver<'static, hal::peripherals::USBD>>) {
    usb.run().await;
}

// If you are trying this and your USB device doesn't connect, the most
// common issues are the RCC config and vbus_detection
//
// See https://embassy.dev/book/#_the_usb_examples_are_not_working_on_my_board_is_there_anything_else_i_need_to_configure
// for more information.
#[embassy_executor::main(entry = "qingke_rt::entry")]
async fn main(spawner: Spawner) {
    let p = hal::init(hal::Config {
        rcc: hal::rcc::Config::SYSCLK_FREQ_144MHZ_HSI,
        ..Default::default()
    });

    let driver = Driver::new(p.USBD, Irqs, p.PA12, p.PA11);

    // Create embassy-usb Config
    let mut config = embassy_usb::Config::new(0xC0DE, 0xCAFE);
    config.manufacturer = Some("Embassy");
    config.product = Some("USB-serial example");
    config.serial_number = Some("12345678");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    // Windows compatibility requires these; CDC-ACM
    config.device_class = 0x02;
    config.device_sub_class = 0x02;
    config.device_protocol = 0x00;
    config.composite_with_iads = false;

    // Allocate buffers for the USB stack
    static CONFIG_DESC: StaticCell<[u8; 256]> = StaticCell::new();
    static BOS_DESC: StaticCell<[u8; 256]> = StaticCell::new();
    static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();
    static CDC_STATE: StaticCell<State> = StaticCell::new();

    let config_desc = CONFIG_DESC.init([0; 256]);
    let bos_desc = BOS_DESC.init([0; 256]);
    let control_buf = CONTROL_BUF.init([0; 64]);
    let cdc_state = CDC_STATE.init(State::new());

    // Build the USB device
    let mut builder = embassy_usb::Builder::new(
        driver,
        config,
        config_desc,
        bos_desc,
        &mut [], // no msos descriptors
        control_buf,
    );
    // Create CDC ACM class
    let mut cdc = CdcAcmClass::new(&mut builder, cdc_state, 64);

    // Build and spawn the USB device task
    let usb = builder.build();
    spawner.spawn(cdc_task(cdc).unwrap());
    spawner.spawn(usb_task(usb).unwrap());

    let pin = Output::new(p.PA4, Level::Low, Speed::High);
    let mut led = Ws2812::new(Delay, pin);

    let mut color = Hsv {
        hue: 40,
        sat: 255,
        val: 5,
    };


    loop {

            color.hue = color.hue.wrapping_add(10);
            let _ = led.write(once(hsv2rgb(color)));
            // Write to USB CDC; break if disconnected
            let _ = LOGGING_PIPE.try_write(b"grelp\n\r");

            Timer::after_millis(50).await;

    }
}
