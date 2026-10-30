use hal::gpio::{AnyPin, Level, Output};
use hal::prelude::*;
use hal::spi::Spi;
use hal::{peripherals, println, Peri};
use {ch32_hal as hal, panic_halt as _};

use st25r95::{
    iso14443a::TransmissionFlags,
    St25r95Spi,
    St25r95Gpio,
    St25r95,
};