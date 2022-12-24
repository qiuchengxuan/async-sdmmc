#[cfg(feature = "async")]
use alloc::boxed::Box;
use core::slice;
#[cfg(not(feature = "fugit"))]
use core::time::Duration;

use embedded_hal::{digital::v2::OutputPin, timer::CountDown};
#[cfg(feature = "fugit")]
use fugit::NanosDurationU32 as Duration;

use crate::{
    bus::Write,
    sd::{command::Command, data, BLOCK_SIZE},
};

use super::bus::{millis, BUSError, Bus, Error, Transfer};

#[cfg_attr(feature = "async", async_trait::async_trait)]
#[cfg_attr(not(feature = "async"), deasync::deasync)]
impl<E, F, SPI, CS, C> Write for Bus<SPI, CS, C>
where
    SPI: Transfer<Error = E> + Send,
    CS: OutputPin<Error = F> + Send,
    C: CountDown<Time = Duration> + Send,
{
    type Error = Error<E, F>;

    async fn write<'a, B>(&mut self, address: u32, blocks: B) -> Result<(), BUSError<E, F>>
    where
        B: core::iter::ExactSizeIterator<Item = &'a [u8; BLOCK_SIZE]> + Send,
    {
        self.tx(&[0xFF; 5]).await?;
        self.select()?;
        let num_blocks = blocks.len();
        let (cmd, token) = match num_blocks {
            1 => (Command::WriteBlock(address), data::Token::Start),
            _ => (Command::WriteMultipleBlock(address), data::Token::StartWriteMultipleBlock),
        };
        self.send_command(cmd).await?;
        for block in blocks {
            self.tx(&[token as u8]).await?;
            self.tx(block).await?;
            let crc = [0u8; 2];
            self.tx(&crc).await?;
            let mut byte = 0u8;
            self.rx(slice::from_mut(&mut byte)).await?;
            match data::Response::try_from(byte) {
                Some(data::Response::Accepted) => (),
                Some(_) => return Err(BUSError::Transfer(data::Error::Generic)),
                None => return Err(BUSError::Generic),
            }
            self.wait(millis(250)).await?;
        }
        if num_blocks > 1 {
            self.tx(&[data::Token::Stop as u8, 0xFF]).await?;
            self.wait(millis(250)).await?;
        }
        self.deselect()?;
        self.tx(&[0xFF]).await?; // Extra byte to release MISO
        Ok(())
    }
}
