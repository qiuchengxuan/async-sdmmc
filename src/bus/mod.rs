#[cfg(feature = "async")]
use alloc::boxed::Box;

use crate::sd::{data, registers::CSD, response::R1Status, BLOCK_SIZE};

#[derive(Debug)]
pub enum Error<BUS> {
    BUS(BUS),
    NoResponse,            // Probably no card
    NotIdle,               // Not idle
    Command(R1Status),     // Command related error
    Transfer(data::Error), // R/W error
    Timeout,               // No respond within expected duration
    Generic,               // Unexpected error
}

pub trait Bus {
    type Error;
    fn before(&mut self) -> Result<(), Error<Self::Error>>;
    fn after(&mut self) -> Result<(), Error<Self::Error>>;
}

#[cfg_attr(feature = "async", async_trait::async_trait)]
#[cfg_attr(not(feature = "async"), deasync::deasync)]
pub trait Read {
    type Error;
    async fn read_csd(&mut self) -> Result<CSD, Error<Self::Error>>;
    async fn read<'a, B>(&mut self, block: u32, blocks: B) -> Result<(), Error<Self::Error>>
    where
        B: core::iter::ExactSizeIterator<Item = &'a mut [u8; BLOCK_SIZE]> + Send;
}

#[cfg_attr(feature = "async", async_trait::async_trait)]
#[cfg_attr(not(feature = "async"), deasync::deasync)]
pub trait Write {
    type Error;
    async fn write<'a, B>(&mut self, block: u32, blocks: B) -> Result<(), Error<Self::Error>>
    where
        B: core::iter::ExactSizeIterator<Item = &'a [u8; BLOCK_SIZE]> + Send;
}

pub mod spi;
