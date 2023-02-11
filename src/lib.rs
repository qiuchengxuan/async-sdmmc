//! # sdmmc
//!
//! > A sdmmc implementation mainly focusing on embedded system with `no_std` and `async` support
//!
//! ## Using this crate
//!
//! Assuming you already have `SPI` struct which implements `sdmmc::spi::Transfer`
//!
//! ```rust
//! let mut bus = sdmmc::bus::linux::spi(&args.spi, args.cs).map_err(|e| format!("{:?}", e))?;
//! let card = bus.init(Delay).await.map_err(|e| format!("{:?}", e))?;
//! debug!("Card: {:?}", card);
//! let mut sd = SD::init(spi, card).await.map_err(|e| format!("{:?}", e))?;
//! let size = Size::from_bytes(sd.num_blocks() as u64 * sd.block_size() as u64);
//! debug!("Size {}", size);
//!
//! let options = SpidevOptions { max_speed_hz: Some(2_000_000), ..Default::default() };
//! sd.bus(|bus| bus.spi(|spi| spi.0.configure(&options))).unwrap();
//!
//! let mut buffer = [0u8; 512];
//! sd.read(0, slice::from_mut(&mut buffer).iter_mut()).await.map_err(|e| format!("{:?}", e))?;
//! let mbr = MasterBootRecord::from_bytes(&buffer).map_err(|e| format!("{:?}", e))?;
//! for partition in mbr.partition_table_entries().iter() {
//!     println!("{:?}", partition);
//! }
//! Ok(())
//! ```

#![cfg_attr(not(any(test, feature = "std")), no_std)]

extern crate alloc;
#[macro_use]
extern crate log;
#[cfg(feature = "spidev")]
extern crate spidev;

pub mod bus;
pub mod delay;
mod sd;

use bus::Error;
pub use sd::registers::NumBlocks;
use sd::{registers::CSD, BLOCK_SIZE};

pub struct SD<BUS> {
    bus: BUS,
    card: sd::Card,
    csd: CSD,
}

type LBA = u32;

#[cfg_attr(not(feature = "async"), deasync::deasync)]
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
