#[cfg(feature = "linux-spi")]
pub mod linux;
pub mod spi;

use crate::sd::{registers::CSD, response::R1Status, transfer, BLOCK_SIZE};

#[derive(Debug)]
pub enum Error<BUS> {
    /// Bus error
    BUS(BUS),
    /// Probably no card
    NoResponse,
    /// Not idle
    NotIdle,
    /// Command related error
    Command(R1Status),
    /// Transfer error
    Transfer(transfer::TokenError),
    /// No respond within expected duration
    Timeout,
    /// Unexpected error
    Generic,
}

impl<BUS> From<BUS> for Error<BUS> {
    fn from(error: BUS) -> Self {
        Self::BUS(error)
    }
}

pub trait Bus {
    type Error;
    fn before(&mut self) -> Result<(), Error<Self::Error>>;
    fn after(&mut self) -> Result<(), Error<Self::Error>>;
}

pub trait Read {
    type Error;
    async fn read_csd(&mut self) -> Result<CSD, Error<Self::Error>>;
    async fn read<'a, B>(&mut self, block: u32, blocks: B) -> Result<(), Error<Self::Error>>
    where
        B: core::iter::ExactSizeIterator<Item = &'a mut [u8; BLOCK_SIZE]> + Send;
}

pub trait Write {
    type Error;

    async fn write<'a, B>(&mut self, block: u32, blocks: B) -> Result<(), Error<Self::Error>>
    where
        B: core::iter::ExactSizeIterator<Item = &'a [u8; BLOCK_SIZE]> + Send;
}
