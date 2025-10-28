#[macro_use]
extern crate log;

use std::slice;

#[cfg(feature = "async")]
use async_std::task;
use clap::Parser;
use mbr_nostd::{MasterBootRecord, PartitionTable};
use sdmmc::delay::std::Delay;
use sdmmc::SD;
use size::Size;
use spidev::SpidevOptions;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    quiet: bool,
    /// Verbosity
    #[clap(short, action = clap::ArgAction::Count)]
    verbosity: u8,
    /// Specify SPI device
    spi: String,
    /// Specify chip-select GPIO number
    cs: u16,
}

#[cfg_attr(not(feature = "async"), deasync::deasync)]
async fn run(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let mut bus = sdmmc::bus::linux::spi(&args.spi, args.cs)?;
    let card = bus.init(Delay).await?;
    debug!("Card: {:?}", card);
    let mut sd = SD::init(bus, card).await?;
    let num_blocks: u64 = sd.num_blocks().into();
    let size = Size::from_bytes(num_blocks * (1 << sd.block_size_shift()));
    debug!("Size {}", size);

    let options = SpidevOptions { max_speed_hz: Some(2_000_000), ..Default::default() };
    sd.bus(|bus| bus.spi(|spi| spi.0.configure(&options))).unwrap();

    let mut buffer = [0u8; 512];
    sd.read(0, slice::from_mut(&mut buffer).iter_mut()).await?;
    let mbr = MasterBootRecord::from_bytes(&buffer).map_err(|e| format!("{:?}", e))?;
    for partition in mbr.partition_table_entries().iter() {
        println!("{:?}", partition);
    }
    Ok(())
}

fn main() {
    let args = Args::parse();
    let level = match (args.quiet, args.verbosity) {
        (true, _) => log::LevelFilter::Off,
        (_, 0) => log::LevelFilter::Info,
        (_, 1) => log::LevelFilter::Debug,
        (_, _) => log::LevelFilter::Trace,
    };
    log::set_max_level(level);
    env_logger::builder().filter(None, level).target(env_logger::Target::Stdout).init();
    let result = match () {
        #[cfg(feature = "async")]
        () => task::block_on(run(&args)),
        #[cfg(not(feature = "async"))]
        () => run(&args),
    };
    match result {
        Ok(_) => (),
        Err(e) => println!("{}", e),
    };
}
