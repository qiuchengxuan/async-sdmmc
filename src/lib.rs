// #![doc = include_str!("../README.md")]
#![cfg_attr(not(any(test, feature = "std")), no_std)]

#![cfg(nightly)]

#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

#[cfg(feature = "async-trait")]
extern crate alloc;

pub mod bus;
pub mod delay;
mod sd;

pub use sd::registers::NumBlocks;

use bus::Error;
use sd::{registers::CSD, BLOCK_SIZE};

pub struct SD<BUS> {
    bus: BUS,
    card: sd::Card,
    csd: CSD,
}

type LBA = u32;

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

    pub fn bus<R>(&mut self, f: impl Fn(&mut BUS) -> R) -> R {
        f(&mut self.bus)
    }

    pub async fn read<'a, B>(&mut self, address: LBA, blocks: B) -> Result<(), Error<E>>
    where
        B: core::iter::ExactSizeIterator<Item = &'a mut [u8; BLOCK_SIZE]> + Send,
    {
        if blocks.len() == 0 {
            return Ok(());
        }
        self.bus.before()?;
        let address = if self.card.high_capacity() { address } else { address * BLOCK_SIZE as u32 };
        let result = self.bus.read(address, blocks).await;
        self.bus.after().and(result)
    }

    pub async fn write<'a, B>(&mut self, address: LBA, blocks: B) -> Result<(), Error<E>>
    where
        B: core::iter::ExactSizeIterator<Item = &'a [u8; BLOCK_SIZE]> + Send,
    {
        if blocks.len() == 0 {
            return Ok(());
        }
        let address = if self.card.high_capacity() { address } else { address * BLOCK_SIZE as u32 };
        self.bus.before()?;
        let result = self.bus.write(address, blocks).await;
        self.bus.after().and(result)
    }

    pub fn num_blocks(&self) -> NumBlocks {
        self.csd.num_blocks()
    }

    pub fn block_size_shift(&self) -> u8 {
        self.csd.block_size_shift()
    }
}
