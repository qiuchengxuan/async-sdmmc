use std::{io, time};

use gpio::{sysfs::SysFsGpioOutput, GpioOut};
use spidev::{SpiModeFlags, Spidev, SpidevOptions, SpidevTransfer};

use crate::bus::spi;

pub struct SPI(pub Spidev);

#[cfg_attr(feature = "async", async_trait::async_trait)]
#[cfg_attr(not(feature = "async"), deasync::deasync)]
impl spi::Transfer for SPI {
    type Error = io::Error;

    async fn transfer(&mut self, tx: &[u8], rx: &mut [u8]) -> io::Result<()> {
        let mut buf = Vec::with_capacity(std::cmp::max(tx.len(), rx.len()));
        if tx.len() > 0 {
            buf.resize(tx.len(), 0);
            self.0.transfer(&mut SpidevTransfer::read_write(tx, &mut buf))
        } else {
            buf.resize(rx.len(), 0xFF);
            self.0.transfer(&mut SpidevTransfer::read_write(&buf, rx))
        }
    }
}

pub struct GPIO(SysFsGpioOutput);

impl embedded_hal::digital::v2::OutputPin for GPIO {
    type Error = io::Error;

    fn set_high(&mut self) -> io::Result<()> {
        self.0.set_value(true)
    }

    fn set_low(&mut self) -> io::Result<()> {
        self.0.set_value(false)
    }
}

impl SPI {
    pub fn new(spi: &str) -> io::Result<Self> {
        let mut spi = Spidev::open(spi)?;
        let options = SpidevOptions::new()
            .bits_per_word(8)
            .max_speed_hz(200_000)
            .mode(SpiModeFlags::SPI_MODE_0)
            .build();
        spi.configure(&options)?;
        Ok(Self(spi))
    }
}

pub struct CountDown(time::Instant);

impl Default for CountDown {
    fn default() -> Self {
        Self(time::Instant::now())
    }
}

impl embedded_hal::timer::CountDown for CountDown {
    type Time = core::time::Duration;

    fn start<T: Into<core::time::Duration>>(&mut self, duration: T) {
        self.0 = time::Instant::now() + duration.into();
    }

    fn wait(&mut self) -> nb::Result<(), void::Void> {
        match time::Instant::now() > self.0 {
            true => Ok(()),
            false => Err(nb::Error::WouldBlock),
        }
    }
}

pub fn spi(spi: &str, cs: u16) -> io::Result<spi::Bus<SPI, GPIO, CountDown>> {
    let spi = SPI::new(spi)?;
    let cs = gpio::sysfs::SysFsGpioOutput::open(cs)?;
    Ok(spi::Bus::new(spi, GPIO(cs), CountDown::default()))
}
