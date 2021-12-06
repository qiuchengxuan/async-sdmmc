use alloc::boxed::Box;
use core::convert::TryFrom;
use core::slice;
use core::time::Duration;

use async_trait::async_trait;
use embedded_hal::digital::v2::OutputPin;
use embedded_hal::timer::CountDown;

use crate::bus::Read;
use crate::sd::command::Command;
use crate::sd::data;
use crate::sd::registers::CSD;
use crate::sd::BLOCK_SIZE;

use super::bus::{AsyncSPI, BUSError, Bus, Error};

impl<E, F, SPI, CS, C> Bus<SPI, CS, C>
where
    SPI: AsyncSPI<Error = E> + Send,
    CS: OutputPin<Error = F> + Send,
    C: CountDown<Time = Duration> + Send,
{
    pub(crate) async fn read_block(&mut self, block: &mut [u8]) -> Result<(), BUSError<E, F>> {
        self.countdown.start(Duration::from_millis(100));
        let token = loop {
            if self.countdown.wait().is_ok() {
                return Err(BUSError::Timeout);
            }
            let mut byte = 0u8;
            self.rx(slice::from_mut(&mut byte)).await?;
            if byte == 0xFF {
                continue;
            }
            match data::Token::try_from(byte) {
                Ok(token) => break token,
                Err(data::Error::NotToken) => continue,
                Err(e) => return Err(BUSError::Transfer(e)),
            }
        };
        if token != data::Token::Start {
            return Err(BUSError::Generic);
        }
        self.rx(block).await?;
        let mut crc = [0u8; 2];
        self.rx(&mut crc).await
    }
}

#[async_trait]
impl<E, F, SPI, CS, C> Read for Bus<SPI, CS, C>
where
    SPI: AsyncSPI<Error = E> + Send,
    CS: OutputPin<Error = F> + Send,
    C: CountDown<Time = Duration> + Send,
{
    type Error = Error<E, F>;

    async fn read_csd(&mut self) -> Result<CSD, BUSError<E, F>> {
        self.tx(&[0xFF; 5]).await?;
        self.select()?;
        self.send_command(Command::SendCSD(0)).await?;
        let mut buffer = [0u8; 16];
        self.read_block(&mut buffer).await?;
        self.deselect()?;
        self.tx(&[0xFF]).await?; // Extra byte to release MISO
        CSD::try_from(u128::from_be_bytes(buffer)).ok_or(BUSError::Generic)
    }

    async fn read(&mut self, address: u32, output: &mut [u8]) -> Result<(), BUSError<E, F>> {
        self.tx(&[0xFF; 5]).await?;
        self.select()?;
        let cmd = if output.len() <= BLOCK_SIZE {
            Command::ReadSingleBlock(address)
        } else {
            Command::ReadMultipleBlock(address)
        };
        self.send_command(cmd).await?;
        for chunk in output.chunks_mut(BLOCK_SIZE) {
            self.read_block(chunk).await?;
        }
        if output.len() > BLOCK_SIZE {
            self.send_command(Command::StopTransmission).await?;
            self.wait(Duration::from_millis(100)).await?;
        }
        self.deselect()?;
        self.tx(&[0xFF]).await // Extra byte to release MISO
    }
}
