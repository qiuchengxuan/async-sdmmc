use core::slice;
#[cfg(not(feature = "fugit"))]
use core::time::Duration;

use embedded_hal::{digital::v2::OutputPin, timer::CountDown};

use embedded_hal_async::spi::{ErrorType, SpiBus, SpiDevice};
#[cfg(feature = "fugit")]
use fugit::NanosDurationU32 as Duration;

use crate::{
    bus::{
        self,
        spi::bus::{millis, Bus, Error},
        Write,
    },
    sd::{
        command::Command,
        transfer::{Response, Token, TokenError},
        BLOCK_SIZE,
    },
};

impl<E, F, SPI, CS, C> Write for Bus<SPI, CS, C>
where
    SPI: SpiDevice + ErrorType<Error = E>,
    SPI::Bus: SpiBus,
    CS: OutputPin<Error = F> + Send,
    C: CountDown<Time = Duration> + Send,
{
    type Error = Error<E, F>;

    async fn write<'a, B>(&mut self, address: u32, blocks: B) -> Result<(), bus::Error<Self::Error>>
    where
        B: core::iter::ExactSizeIterator<Item = &'a [u8; BLOCK_SIZE]> + Send,
    {
        self.tx(&[0xFF; 5]).await?;
        self.select()?;
        let num_blocks = blocks.len();
        let (cmd, token) = match num_blocks {
            1 => (Command::WriteBlock(address), Token::Start),
            _ => (Command::WriteMultipleBlock(address), Token::StartWriteMultipleBlock),
        };
        self.send_command(cmd).await?;
        for block in blocks {
            self.tx(&[token as u8]).await?;
            self.tx(block).await?;
            let crc = [0u8; 2];
            self.tx(&crc).await?;
            let mut byte = 0u8;
            self.rx(slice::from_mut(&mut byte)).await?;
            match Response::try_from(byte) {
                Some(Response::Accepted) => (),
                Some(_) => return Err(bus::Error::Transfer(TokenError::Generic)),
                None => return Err(bus::Error::Generic),
            }
            self.wait(millis(250)).await?;
        }
        if num_blocks > 1 {
            self.tx(&[Token::Stop as u8, 0xFF]).await?;
            self.wait(millis(250)).await?;
        }
        self.deselect()?;
        self.tx(&[0xFF]).await?; // Extra byte to release MISO
        Ok(())
    }
}
