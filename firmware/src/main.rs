#![no_std]
#![no_main]

pub mod cdcdebug;
use crate::cdcdebug::cdc_task;

use ch32_hal::peripherals;
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_time::{Duration, Timer};
use embassy_usb::UsbDevice;
use hal::usbd::{Driver };
use hal::{bind_interrupts};
use {ch32_hal as hal, panic_halt as _};
use static_cell::StaticCell;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::pipe::Pipe;
use embassy_usb::class::hid::{HidReaderWriter, ReportId, RequestHandler, State as HidState};
use embassy_usb::class::cdc_acm::{CdcAcmClass, State as CdcState};
use embassy_usb::control::OutResponse;

use core::iter::once;
use hal::delay::Delay;
use hal::gpio::{Level, Output, Speed};
use smart_leds::hsv::{hsv2rgb, Hsv};
use smart_leds::SmartLedsWrite;
use ws2812_delay::Ws2812;

bind_interrupts!(struct Irqs {
    USB_LP_CAN1_RX0 => hal::usbd::InterruptHandler<hal::peripherals::USBD>;
});

const FIDO2_HID_DESC: &[u8] = &[
    0x06, 0xD0, 0xF1,  // Usage Page (FIDO)
    0x09, 0x01,        // Usage (CTAPHID)
    0xA1, 0x01,        // Collection (Application)
    0x09, 0x20,        // Usage (Data In)
    0x15, 0x00,        // Logical min (0)
    0x26, 0xFF, 0x00,  // Logical max (255)
    0x75, 0x08,        // Report Size (8)
    0x95, 0x40,        // Report count (64)
    0x81, 0x02,        // Input (Data,Var,Abs)
    0x09, 0x21,        // Usage (Data Out)
    0x15, 0x00,        // Logical min (0)
    0x26, 0xFF, 0x00,  // Logical max (255)
    0x75, 0x08,        // Report Size (8)
    0x95, 0x40,        // Report count (64)
    0x91, 0x02,        // Output (Data,Var,Abs)
    0xC0,              // End Collection
];


pub static LOGGING_PIPE: Pipe<CriticalSectionRawMutex, 64> = Pipe::new();

#[embassy_executor::task]
async fn hid_task(
    hid: HidReaderWriter<'static, Driver<'static, peripherals::USBD>, 1, 1>,
) {
    let (mut reader, mut writer) = hid.split();

    loop {
                   Timer::after_millis(50).await;

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

        // Composite device: class defined at interface level
    config.device_class = 0xEF;    // Miscellaneous
    config.device_sub_class = 0x02; // Common Class
    config.device_protocol = 0x01;  // Interface Association Descriptor

    // Allocate buffers for the USB stack
    static CONFIG_DESC: StaticCell<[u8; 256]> = StaticCell::new();
    static BOS_DESC: StaticCell<[u8; 256]> = StaticCell::new();
    static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();
    static CDC_STATE: StaticCell<CdcState> = StaticCell::new();
    static HID_STATE: StaticCell<HidState> = StaticCell::new();



    let config_desc = CONFIG_DESC.init([0; 256]);
    let bos_desc = BOS_DESC.init([0; 256]);
    let control_buf = CONTROL_BUF.init([0; 64]);
    let cdc_state = CDC_STATE.init(CdcState::new());
    let hid_state = HID_STATE.init(HidState::new());


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

        // Add HID class (keyboard)
    let hid_config = embassy_usb::class::hid::Config {
        hid_subclass: embassy_usb::class::hid::HidSubclass::No,
        report_descriptor: FIDO2_HID_DESC,
        request_handler: None,
        poll_ms: 5,
        max_packet_size: 64,
        hid_boot_protocol: embassy_usb::class::hid::HidBootProtocol::None,
    };
    let hid = HidReaderWriter::<_, 1, 1>::new(&mut builder, hid_state, hid_config);



    // Build and spawn the USB device task
    let usb = builder.build();
    spawner.spawn(hid_task(hid).unwrap());
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


            Timer::after_millis(50).await;

    }
}
