use core::future::Future;
use std::{
    io::{self, ErrorKind as IoKind, Read, Write},
    time,
};

use gpio::{sysfs::SysFsGpioOutput, GpioOut};
use spidev::{SpiModeFlags, Spidev, SpidevOptions, SpidevTransfer};

use embedded_hal_async::spi::{
    Error as SpiError, ErrorKind, ErrorType, SpiBus, SpiBusFlush, SpiBusRead, SpiBusWrite,
    SpiDevice,
};

use crate::bus::spi;

#[derive(Debug)]
pub struct Error(std::io::Error);

impl SpiError for Error {
    fn kind(&self) -> ErrorKind {
        match self.0.kind() {
            IoKind::OutOfMemory => ErrorKind::Overrun,
            IoKind::Other => ErrorKind::Other,
            _ => ErrorKind::Other,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(io_error: std::io::Error) -> Self {
        Self(io_error)
    }
}

pub struct SPI(pub Spidev);

impl ErrorType for SPI {
    type Error = Error;
}
impl SpiBusRead for SPI {
    async fn read(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        Ok(self.0.read(words).map(|_| ())?)
    }
}

impl SpiBusWrite for SPI {
    async fn write(&mut self, words: &[u8]) -> Result<(), Self::Error> {
        Ok(self.0.write(words).map(|_| ())?)
    }
}

impl SpiBusFlush for SPI {
    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(self.0.flush()?)
    }
}

impl SpiBus for SPI {
    async fn transfer<'a>(
        &'a mut self,
        read: &'a mut [u8],
        write: &'a [u8],
    ) -> Result<(), Self::Error> {
        let mut buf = Vec::with_capacity(std::cmp::max(read.len(), write.len()));
        if write.len() > 0 {
            buf.resize(write.len(), 0);
            Ok(self.0.transfer(&mut SpidevTransfer::read_write(write, &mut buf))?)
        } else {
            buf.resize(read.len(), 0xFF);
            Ok(self.0.transfer(&mut SpidevTransfer::read_write(&buf, read))?)
        }
    }

    async fn transfer_in_place<'a>(&'a mut self, words: &'a mut [u8]) -> Result<(), Self::Error> {
        todo!()
    }
}

unsafe impl SpiDevice for SPI {
    type Bus = Self;

    async fn transaction<R, F, Fut>(&mut self, f: F) -> Result<R, Self::Error>
    where
        F: FnOnce(*mut Self::Bus) -> Fut,
        Fut: Future<Output = Result<R, <Self::Bus as ErrorType>::Error>>,
    {
        f(self).await
        // let bus = unsafe { &mut *self };

        // let result = async move {
        //     let result = f(bus);
        //     let bus = bus; // Ensure that the bus reference was not moved out of the closure
        //     let _ = bus; // Silence the "unused variable" warning from previous line
        //     result
        // }
        // .await;

        // result.await
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
