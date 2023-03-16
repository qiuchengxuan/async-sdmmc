use core::slice;
#[cfg(not(feature = "fugit"))]
use core::time::Duration;

use embedded_hal::{digital::v2::OutputPin, timer::CountDown};

use embedded_hal_async::spi::{ErrorType, SpiDevice, SpiBus};
#[cfg(feature = "fugit")]
use fugit::NanosDurationU32 as Duration;

use log::trace;

use crate::{
    bus,
    sd::{
        command::{AppCommand, Command},
        response::{self, Response},
    },
};

#[derive(Debug)]
pub enum Error<SPI, CS> {
    SPI(SPI),
    CS(CS),
}

pub type BUSError<SPI, CS> = bus::Error<Error<SPI, CS>>;

// pub trait Transfer {
//     type Error;

//     async fn transfer(&mut self, tx: &[u8], rx: &mut [u8]) -> Result<(), Self::Error>;
// }

// impl<T: embedded_hal_async::spi::SpiBus<u8>> Transfer for T {
//     type Error = <T as embedded_hal_async::spi::ErrorType>::Error;

//     async fn transfer(&mut self, tx: &[u8], rx: &mut [u8]) -> Result<(), Self::Error> {
//         <Self as embedded_hal_async::spi::SpiBus<u8>>::transfer(self, rx, tx).await
//     }
// }

pub struct Bus<SPI, CS, C> {
    spi: SPI,
    cs: CS,
    pub(crate) countdown: C,
}

impl<E, SPI, CS, C> Bus<SPI, CS, C>
where
    CS: OutputPin<Error = E>,
    C: CountDown<Time = Duration>,
{
    pub fn new(spi: SPI, cs: CS, countdown: C) -> Self {
        Self { spi, cs, countdown }
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

impl<E, F, SPI, CS, C> Bus<SPI, CS, C>
where
    SPI: SpiDevice + ErrorType<Error = E>,
    SPI::Bus: SpiBus,
    CS: OutputPin<Error = F>,
    C: CountDown<Time = Duration>,
{
    pub(crate) async fn tx(&mut self, bytes: &[u8]) -> Result<(), BUSError<E, F>> {
        self.spi.transfer(&mut [], bytes).await.map_err(|e| BUSError::BUS(Error::SPI(e)))
    }

    pub(crate) async fn rx(&mut self, buffer: &mut [u8]) -> Result<(), BUSError<E, F>> {
        self.spi.transfer(buffer, &[]).await.map_err(|e| BUSError::BUS(Error::SPI(e)))
    }

    pub(crate) async fn wait(&mut self, timeout: Duration) -> Result<(), BUSError<E, F>> {
        self.countdown.start(timeout);
        let mut byte = 0u8;
        while byte != 0xFFu8 {
            if self.countdown.wait().is_ok() {
                return Err(BUSError::Timeout);
            }
            self.rx(slice::from_mut(&mut byte)).await?;
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

impl<E, F, SPI, CS, C> bus::Bus for Bus<SPI, CS, C>
where
    SPI: SpiDevice + ErrorType<Error = E>,
    CS: OutputPin<Error = F>,
    C: CountDown<Time = Duration>,
{
    type Error = Error<E, F>;

    fn before(&mut self) -> Result<(), BUSError<E, F>> {
        Ok(())
    }

    fn after(&mut self) -> Result<(), BUSError<E, F>> {
        self.deselect()
    }
}

pub(crate) fn millis(millis: u32) -> Duration {
    match () {
        #[cfg(not(feature = "fugit"))]
        () => Duration::from_millis(millis as u64),
        #[cfg(feature = "fugit")]
        () => Duration::millis(millis),
    }
}
