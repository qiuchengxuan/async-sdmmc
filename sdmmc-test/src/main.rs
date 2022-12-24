extern crate spidev;
#[macro_use]
extern crate log;

#[cfg(feature = "async")]
use std::future;
use std::{io, slice, thread, time};

#[cfg(feature = "async")]
use async_std::task;
use clap::Parser;
use gpio::{sysfs::SysFsGpioOutput, GpioOut};
use mbr_nostd::{MasterBootRecord, PartitionTable};
use sdmmc::{bus::spi, SD};
use size::Size;
use spidev::{SpiModeFlags, Spidev, SpidevOptions, SpidevTransfer};

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

struct AsyncSPI(Spidev);

#[cfg_attr(feature = "async", async_trait::async_trait)]
#[cfg_attr(not(feature = "async"), deasync::deasync)]
impl spi::Transfer for AsyncSPI {
    type Error = io::Error;

    async fn transfer(&mut self, tx: &[u8], rx: &mut [u8]) -> io::Result<()> {
        let mut buf = Vec::with_capacity(std::cmp::max(tx.len(), rx.len()));
        if tx.len() > 0 {
            buf.resize(tx.len(), 0);
            self.0.transfer(&mut SpidevTransfer::read_write(tx, &mut buf))
        } else {
            buf.resize(rx.len(), 0xFF);
            self.0.transfer(&mut SpidevTransfer::read_write(&buf, rx))
        }
    }
}

struct GPIO(SysFsGpioOutput);

impl embedded_hal::digital::v2::OutputPin for GPIO {
    type Error = io::Error;

    fn set_high(&mut self) -> io::Result<()> {
        self.0.set_value(true)
    }

    fn set_low(&mut self) -> io::Result<()> {
        self.0.set_value(false)
    }
}

fn spidev(spi: &str) -> io::Result<Spidev> {
    let mut spi = Spidev::open(spi)?;
    let options = SpidevOptions::new()
        .bits_per_word(8)
        .max_speed_hz(200_000)
        .mode(SpiModeFlags::SPI_MODE_0)
        .build();
    spi.configure(&options)?;
    Ok(spi)
}

struct CountDown(time::Instant);

impl Default for CountDown {
    fn default() -> Self {
        Self(time::Instant::now())
    }
}

impl embedded_hal::timer::CountDown for CountDown {
    type Time = core::time::Duration;

    fn start<T: Into<core::time::Duration>>(&mut self, duration: T) {
        self.0 = time::Instant::now() + duration.into();
    }

    fn wait(&mut self) -> nb::Result<(), void::Void> {
        match time::Instant::now() > self.0 {
            true => Ok(()),
            false => Err(nb::Error::WouldBlock),
        }
    }
}

struct Delay;

#[cfg(feature = "async")]
impl<UXX: Into<u64>> sdmmc::delay::Delay<UXX> for Delay {
    type Future = future::Ready<()>;

    fn delay_ms(&mut self, ms: UXX) -> Self::Future {
        thread::sleep(time::Duration::from_millis(ms.into()));
        future::ready(())
    }
}

#[cfg(not(feature = "async"))]
impl<UXX: Into<u64>> embedded_hal::blocking::delay::DelayMs<UXX> for Delay {
    fn delay_ms(&mut self, ms: UXX) {
        thread::sleep(time::Duration::from_millis(ms.into()));
    }
}

#[cfg_attr(not(feature = "async"), deasync::deasync)]
async fn run(args: &Args) -> Result<(), String> {
    let spi = spidev(&args.spi).map_err(|e| e.to_string())?;
    let cs = gpio::sysfs::SysFsGpioOutput::open(args.cs).map_err(|e| e.to_string())?;
    let mut spi = spi::Bus::new(AsyncSPI(spi), GPIO(cs), CountDown::default());
    let card = spi.init(Delay).await.map_err(|e| format!("{:?}", e))?;
    debug!("Card: {:?}", card);
    let mut sd = SD::init(spi, card).await.map_err(|e| format!("{:?}", e))?;
    let size = Size::from_bytes(sd.num_blocks() as u64 * sd.block_size() as u64);
    debug!("Size {}", size);

    let options = SpidevOptions { max_speed_hz: Some(2_000_000), ..Default::default() };
    sd.bus(|bus| bus.spi(|spi| spi.0.configure(&options))).unwrap();

    let mut buffer = [0u8; 512];
    sd.read(0, slice::from_mut(&mut buffer).iter_mut()).await.map_err(|e| format!("{:?}", e))?;
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
