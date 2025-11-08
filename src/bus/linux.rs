use std::{io, time};

use derive_more::Display;
use gpio::{sysfs::SysFsGpioOutput, GpioOut};
use spidev::{SpiModeFlags, Spidev, SpidevOptions, SpidevTransfer};
use thiserror::Error;

use crate::bus::spi;

pub struct SPI(pub Spidev);

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

#[derive(Debug, Display, Error)]
pub struct IOError(#[from] io::Error);

impl embedded_hal::digital::Error for IOError {
    fn kind(&self) -> embedded_hal::digital::ErrorKind {
        embedded_hal::digital::ErrorKind::Other
    }
}

impl embedded_hal::digital::ErrorType for GPIO {
    type Error = IOError;
}

impl embedded_hal::digital::OutputPin for GPIO {
    fn set_high(&mut self) -> Result<(), IOError> {
        Ok(self.0.set_value(true)?)
    }

    fn set_low(&mut self) -> Result<(), IOError> {
        Ok(self.0.set_value(false)?)
    }
}

impl SPI {
    pub fn new(spi: &str) -> io::Result<Self> {
        let mut spi = Spidev::open(spi)?;
        let mode = SpiModeFlags::SPI_MODE_0;
        let mut builder = SpidevOptions::new();
        spi.configure(&builder.bits_per_word(8).max_speed_hz(200_000).mode(mode).build())?;
        Ok(Self(spi))
    }
}

pub struct SystemClock {}

impl embedded_timers::clock::Clock for SystemClock {
    type Instant = embedded_timers::instant::TimespecInstant;

    fn now(&self) -> Self::Instant {
        let now = time::SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap_or_default();
        embedded_timers::instant::TimespecInstant::new(now.as_secs() as u32, now.subsec_nanos())
    }
}

pub fn spi(spi: &str, cs: u16) -> io::Result<spi::Bus<SPI, GPIO, SystemClock>> {
    let spi = SPI::new(spi)?;
    let cs = gpio::sysfs::SysFsGpioOutput::open(cs)?;
    Ok(spi::Bus::new(spi, GPIO(cs), SystemClock {}))
}
