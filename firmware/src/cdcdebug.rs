use crate::LOGGING_PIPE;

use ch32_hal::peripherals;
use embassy_time::{Timer};
use embassy_usb::UsbDevice;
use hal::usbd::{Driver };
use {ch32_hal as hal, panic_halt as _};
use embassy_usb::class::cdc_acm::{CdcAcmClass};

#[embassy_executor::task]
pub async fn cdc_task(
    mut cdc: CdcAcmClass<'static, Driver<'static, peripherals::USBD>>
) {
    loop {
        cdc.wait_connection().await;

        loop {

            if !LOGGING_PIPE.is_empty() {
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