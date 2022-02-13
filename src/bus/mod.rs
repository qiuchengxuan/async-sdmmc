#[cfg(feature = "async")]
use alloc::boxed::Box;

#[cfg(feature = "async")]
use async_trait::async_trait;

use crate::sd::{data, registers::CSD, response::R1Status};

#[derive(Debug)]
pub enum Error<BUS> {
    BUS(BUS),
    NoResponse,            // Probably no card
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

#[cfg_attr(feature = "async", async_trait)]
#[deasync::deasync]
pub trait Read {
    type Error;
    async fn read_csd(&mut self) -> Result<CSD, Error<Self::Error>>;
    async fn read(&mut self, block: u32, buffer: &mut [u8]) -> Result<(), Error<Self::Error>>;
}

#[cfg_attr(feature = "async", async_trait)]
#[deasync::deasync]
pub trait Write {
    type Error;
    async fn write(&mut self, block: u32, buffer: &[u8]) -> Result<(), Error<Self::Error>>;
}

pub mod spi;
