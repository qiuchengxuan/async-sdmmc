#![cfg_attr(not(any(test, feature = "std")), no_std)]

extern crate alloc;

pub mod bus;
pub mod delay;
mod sd;

use bus::Error;
use sd::{registers::CSD, BLOCK_SIZE};

pub struct SD<BUS> {
    bus: BUS,
    card: sd::Card,
    csd: CSD,
}

type LBA = usize;

#[deasync::deasync]
impl<E, BUS> SD<BUS>
where
    BUS: bus::Read<Error = E> + bus::Write<Error = E> + bus::Bus<Error = E>,
{
    pub async fn init(mut bus: BUS, card: sd::Card) -> Result<Self, Error<E>> {
        bus.before()?;
        let result = bus.read_csd().await;
        bus.after()?;
        result.map(|csd| Self { bus, card, csd })
    }

    pub fn csd(&self) -> CSD {
        self.csd
    }

    pub async fn read(&mut self, address: LBA, mut buffer: &mut [u8]) -> Result<usize, Error<E>> {
        let len = buffer.len() / BLOCK_SIZE * BLOCK_SIZE;
        if len == 0 {
            return Ok(0);
        }
        self.bus.before()?;
        buffer = &mut buffer[..len];
        let address = if self.card.high_capacity() { address } else { address * BLOCK_SIZE };
        let result = self.bus.read(address as u32, buffer).await;
        self.bus.after().and(result).map(|_| len)
    }

    pub async fn write(&mut self, address: LBA, mut bytes: &[u8]) -> Result<usize, Error<E>> {
        let len = bytes.len() / BLOCK_SIZE * BLOCK_SIZE;
        if len == 0 {
            return Ok(0);
        }
        bytes = &bytes[..len];
        let address = if self.card.high_capacity() { address } else { address * BLOCK_SIZE };
        self.bus.before()?;
        let result = self.bus.write(address as u32, bytes).await;
        self.bus.after().and(result).map(|_| len)
    }

    pub fn num_blocks(&self) -> usize {
        self.csd.num_blocks()
    }
}
