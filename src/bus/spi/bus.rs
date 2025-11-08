use core::slice;
use core::time::Duration;

use derive_more::Display;
use embedded_hal::digital::OutputPin;
#[cfg(not(feature = "async"))]
use embedded_hal::spi;
#[cfg(all(feature = "async", feature = "embedded-hal-async"))]
use embedded_hal_async::spi::SpiBus;
use embedded_timers::{clock::Clock, instant::Instant};

use crate::sd::{
    command::{AppCommand, Command},
    response::{self, Response},
};

use crate::bus;

#[derive(Debug, Display)]
pub enum Error<SPI, CS> {
    #[display("spi error: {_0}")]
    SPI(SPI),
    #[display("chip select error: {_0}")]
    CS(CS),
}

impl<SPI: core::error::Error, CS: core::error::Error> core::error::Error for Error<SPI, CS> {}

pub type BUSError<SPI, CS> = bus::Error<Error<SPI, CS>>;

pub trait Transfer {
    type Error;
    #[cfg(not(feature = "async"))]
    fn transfer(&mut self, tx: &[u8], rx: &mut [u8]) -> Result<(), Self::Error>;
    #[cfg(feature = "async")]
    fn transfer(
        &mut self,
        tx: &[u8],
        rx: &mut [u8],
    ) -> impl Future<Output = Result<(), Self::Error>>;
}

#[cfg(not(feature = "async"))]
impl<E, T: spi::SpiBus<u8, Error = E>> Transfer for T {
    type Error = E;

    fn transfer(&mut self, tx: &[u8], rx: &mut [u8]) -> Result<(), Self::Error> {
        self.transfer(rx, tx).map(|_| ())
    }
}

#[cfg(all(feature = "async", feature = "embedded-hal-async"))]
impl<E, T: SpiBus<u8, Error = E>> Transfer for T {
    type Error = E;

    async fn transfer(&mut self, tx: &[u8], rx: &mut [u8]) -> Result<(), Self::Error> {
        match (!tx.is_empty(), !rx.is_empty()) {
            (true, true) => self.transfer(rx, tx).await,
            (true, false) => self.read(rx).await,
            (false, true) => self.write(tx).await,
            _ => unreachable!(),
        }
    }
}

pub struct Bus<SPI, CS, C> {
    spi: SPI,
    cs: CS,
    pub(crate) clock: C,
}

impl<E, SPI, CS, C, I> Bus<SPI, CS, C>
where
    CS: OutputPin<Error = E>,
    C: Clock<Instant = I>,
{
    pub fn new(spi: SPI, cs: CS, clock: C) -> Self {
        Self { spi, cs, clock }
    }

    pub fn spi<R>(&mut self, f: impl Fn(&mut SPI) -> R) -> R {
        f(&mut self.spi)
    }

    pub(crate) fn select<T>(&mut self) -> Result<(), BUSError<T, E>> {
        self.cs.set_low().map_err(|e| BUSError::BUS(Error::CS(e)))
    }

    pub(crate) fn deselect<T>(&mut self) -> Result<(), BUSError<T, E>> {
        self.cs.set_high().map_err(|e| BUSError::BUS(Error::CS(e)))
    }
}

#[cfg_attr(not(feature = "async"), deasync::deasync)]
impl<E, F, SPI, CS, C, I> Bus<SPI, CS, C>
where
    SPI: Transfer<Error = E>,
    CS: OutputPin<Error = F>,
    C: Clock<Instant = I>,
    I: Instant,
{
    pub(crate) async fn tx(&mut self, bytes: &[u8]) -> Result<(), BUSError<E, F>> {
        self.spi.transfer(bytes, &mut []).await.map_err(|e| BUSError::BUS(Error::SPI(e)))
    }

    pub(crate) async fn rx(&mut self, buffer: &mut [u8]) -> Result<(), BUSError<E, F>> {
        self.spi.transfer(&[], buffer).await.map_err(|e| BUSError::BUS(Error::SPI(e)))
    }

    pub(crate) async fn wait(&mut self, timeout: Duration) -> Result<(), BUSError<E, F>> {
        let deadline = self.clock.now() + timeout;
        let mut byte = 0u8;
        while byte != 0xFFu8 {
            if self.clock.now() > deadline {
                return Err(BUSError::Timeout);
            }
            let buf = slice::from_mut(&mut byte);
            self.spi.transfer(&[], buf).await.map_err(|e| BUSError::BUS(Error::SPI(e)))?;
        }
        Ok(())
    }

    pub(crate) async fn send_command(&mut self, cmd: Command) -> Result<Response, BUSError<E, F>> {
        let bytes: [u8; 6] = cmd.into();
        trace!("Send CMD {:?} bytes {:X?}", cmd, &bytes);
        self.tx(&bytes[..]).await?;

        if cmd == Command::StopTransmission {
            self.rx(&mut [0u8]).await?; // Skip stuff byte
        }

        // Skip Ncr, 0~8 bytes for SDC, 1~8 bytes for MMC
        let mut r1 = response::R1::default();
        for _ in 0..=8 {
            self.rx(slice::from_mut(&mut r1.0)).await?;
            if r1.valid() {
                break;
            }
        }
        if !r1.valid() {
            return Err(BUSError::NoResponse);
        }

        if let Some(e) = r1.error() {
            return Err(BUSError::Command(e));
        }
        let mut response = Response { r1, ..Default::default() };

        let size = cmd.expected_response_ex_size();
        if size > 0 {
            let mut buffer = [0u8; 4];
            self.rx(&mut buffer[4 - size..]).await?;
            response.ex = u32::from_be_bytes(buffer);
        }
        Ok(response)
    }

    pub(crate) async fn send_app_command(
        &mut self,
        cmd: AppCommand,
    ) -> Result<Response, BUSError<E, F>> {
        self.send_command(Command::AppCommand(0)).await?;
        self.send_command(Command::App(cmd)).await
    }
}

impl<E, F, SPI, CS, C, I> bus::Bus for Bus<SPI, CS, C>
where
    SPI: Transfer<Error = E>,
    CS: OutputPin<Error = F>,
    C: Clock<Instant = I>,
    I: Instant,
{
    type Error = Error<E, F>;

    fn before(&mut self) -> Result<(), BUSError<E, F>> {
        Ok(())
    }

    fn after(&mut self) -> Result<(), BUSError<E, F>> {
        self.deselect()
    }
}
