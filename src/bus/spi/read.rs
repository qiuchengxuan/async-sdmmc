use core::time::Duration;
use core::{convert::TryFrom, slice};

use embedded_hal::digital::OutputPin;
use embedded_timers::{clock::Clock, instant::Instant};

use crate::{
    bus::Read,
    sd::{
        command::Command,
        registers::CSD,
        transfer::{Token, TokenError},
        BLOCK_SIZE,
    },
};

use super::bus::{BUSError, Bus, Error, Transfer};

impl<E, F, SPI, CS, C, I> Bus<SPI, CS, C>
where
    SPI: Transfer<Error = E>,
    CS: OutputPin<Error = F>,
    C: Clock<Instant = I>,
    I: Instant,
{
    #[cfg_attr(not(feature = "async"), deasync::deasync)]
    pub(crate) async fn read_block(&mut self, block: &mut [u8]) -> Result<(), BUSError<E, F>> {
        let deadline = self.clock.now() + Duration::from_millis(100);
        let token = loop {
            if self.clock.now() > deadline {
                return Err(BUSError::Timeout);
            }
            let mut byte = 0u8;
            self.rx(slice::from_mut(&mut byte)).await?;
            if byte == 0xFF {
                continue;
            }
            match Token::try_from(byte) {
                Ok(token) => break token,
                Err(TokenError::NotToken) => continue,
                Err(e) => return Err(BUSError::Transfer(e)),
            }
        };
        if token != Token::Start {
            return Err(BUSError::Generic);
        }
        self.rx(block).await?;
        let mut crc = [0u8; 2];
        self.rx(&mut crc).await
    }
}

#[cfg_attr(not(feature = "async"), deasync::deasync)]
impl<E, F, SPI, CS, C, I> Read for Bus<SPI, CS, C>
where
    SPI: Transfer<Error = E>,
    CS: OutputPin<Error = F>,
    C: Clock<Instant = I>,
    I: Instant,
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

    async fn read<'a, B>(&mut self, address: u32, blocks: B) -> Result<(), BUSError<E, F>>
    where
        B: core::iter::ExactSizeIterator<Item = &'a mut [u8; BLOCK_SIZE]>,
    {
        self.tx(&[0xFF; 5]).await?;
        self.select()?;
        let num_blocks = blocks.len();
        let cmd = match num_blocks {
            1 => Command::ReadSingleBlock(address),
            _ => Command::ReadMultipleBlock(address),
        };
        self.send_command(cmd).await?;
        for block in blocks {
            self.read_block(block).await?;
        }
        if num_blocks > 1 {
            self.send_command(Command::StopTransmission).await?;
            self.wait(Duration::from_millis(100)).await?;
        }
        self.deselect()?;
        self.tx(&[0xFF]).await // Extra byte to release MISO
    }
}
