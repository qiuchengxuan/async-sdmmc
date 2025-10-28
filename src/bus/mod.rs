#[cfg(feature = "linux-spi")]
pub mod linux;
pub mod spi;

#[cfg(all(feature = "async", feature = "async-trait"))]
use alloc::boxed::Box;

use derive_more::Display;
use thiserror::Error;

use crate::sd::{registers::CSD, response::R1Status, transfer, BLOCK_SIZE};

#[derive(Debug, Error, Display)]
pub enum Error<BUS> {
    #[display("bus error: {_0}")]
    BUS(BUS),
    /// Probably no card
    #[display("no response")]
    NoResponse,
    #[display("not idle")]
    NotIdle,
    #[display("command error: {_0}")]
    Command(#[from] R1Status),
    #[display("transfer error: {_0}")]
    Transfer(#[from] transfer::TokenError),
    /// No respond within expected duration
    #[display("timeout error")]
    Timeout,
    #[display("generic error")]
    Generic,
}

pub trait Bus {
    type Error;
    fn before(&mut self) -> Result<(), Error<Self::Error>>;
    fn after(&mut self) -> Result<(), Error<Self::Error>>;
}

#[cfg_attr(all(feature = "async", feature = "async-trait"), async_trait::async_trait)]
#[cfg_attr(not(feature = "async"), deasync::deasync)]
pub trait Read {
    type Error;
    async fn read_csd(&mut self) -> Result<CSD, Error<Self::Error>>;
    async fn read<'a, B>(&mut self, block: u32, blocks: B) -> Result<(), Error<Self::Error>>
    where
        B: core::iter::ExactSizeIterator<Item = &'a mut [u8; BLOCK_SIZE]> + Send;
}

#[cfg_attr(all(feature = "async", feature = "async-trait"), async_trait::async_trait)]
#[cfg_attr(not(feature = "async"), deasync::deasync)]
pub trait Write {
    type Error;
    async fn write<'a, B>(&mut self, block: u32, blocks: B) -> Result<(), Error<Self::Error>>
    where
        B: core::iter::ExactSizeIterator<Item = &'a [u8; BLOCK_SIZE]> + Send;
}
