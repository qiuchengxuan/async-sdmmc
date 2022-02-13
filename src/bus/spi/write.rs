#[cfg(feature = "async")]
use alloc::boxed::Box;
use core::{slice, time::Duration};

#[cfg(feature = "async")]
use async_trait::async_trait;
use embedded_hal::{digital::v2::OutputPin, timer::CountDown};

use crate::{
    bus::Write,
    sd::{command::Command, data, BLOCK_SIZE},
};

use super::bus::{BUSError, Bus, Error, Transfer};

#[cfg_attr(feature = "async", async_trait)]
#[deasync::deasync]
impl<E, F, SPI, CS, C> Write for Bus<SPI, CS, C>
where
    SPI: Transfer<Error = E> + Send,
    CS: OutputPin<Error = F> + Send,
    C: CountDown<Time = Duration> + Send,
{
    type Error = Error<E, F>;

    async fn write(&mut self, address: u32, bytes: &[u8]) -> Result<(), BUSError<E, F>> {
        self.tx(&[0xFF; 5]).await?;
        self.select()?;
        let (cmd, token) = if bytes.len() == BLOCK_SIZE {
            (Command::WriteBlock(address), data::Token::Start)
        } else {
            (Command::WriteMultipleBlock(address), data::Token::StartWriteMultipleBlock)
        };
        self.send_command(cmd).await?;
        for chunk in bytes.chunks(BLOCK_SIZE) {
            self.tx(&[token as u8]).await?;
            self.tx(chunk).await?;
            let crc = [0u8; 2];
            self.tx(&crc).await?;
            let mut byte = 0u8;
            self.rx(slice::from_mut(&mut byte)).await?;
            match data::Response::try_from(byte) {
                Some(data::Response::Accepted) => (),
                Some(_) => return Err(BUSError::Transfer(data::Error::Generic)),
                None => return Err(BUSError::Generic),
            }
            self.wait(Duration::from_millis(250)).await?;
        }
        if bytes.len() > BLOCK_SIZE {
            self.tx(&[data::Token::Stop as u8, 0xFF]).await?;
            self.wait(Duration::from_millis(250)).await?;
        }
        self.deselect()?;
        self.tx(&[0xFF]).await?; // Extra byte to release MISO
        Ok(())
    }
}
