#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_time::Timer;
use embassy_usb::UsbDevice;
use hal::usbd::{Driver };
use hal::{bind_interrupts};
use {ch32_hal as hal, panic_halt as _};
use static_cell::StaticCell;

bind_interrupts!(struct Irqs {
    USB_LP_CAN1_RX0 => hal::usbd::InterruptHandler<hal::peripherals::USBD>;
});

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
    spawner.spawn(usb_task(usb).unwrap());

    loop {
        // Wait for the host to open the serial port
        cdc.wait_connection().await;

        loop {

            // Format the output string
            let msg: [u8; 13] = [0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x20, 0x57, 0x6F, 0x72, 0x6C, 0x64, 0x0A, 0x0D];

            // Write to USB CDC; break if disconnected
            if cdc.write_packet(&msg).await.is_err() {
                break;
            }

            Timer::after_millis(500).await;
        }
    }
}
