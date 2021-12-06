extern crate spidev;

use std::future;
use std::io;
use std::thread;
use std::time;

use async_std::task;
use clap::{App, Arg};
use gpio::{sysfs::SysFsGpioOutput, GpioOut};
use mbr_nostd::{MasterBootRecord, PartitionTable};
use sdmmc::bus::spi;
use sdmmc::delay::AsyncDelay;
use sdmmc::SD;
use spidev::{SpiModeFlags, Spidev, SpidevOptions, SpidevTransfer};

fn parse_args() -> Result<(String, u16), String> {
    let matches = App::new("sdmmc")
        .version("0.1.0")
        .arg(Arg::with_name("spi").long("spi").takes_value(true).help("Specify SPI device"))
        .arg(
            Arg::with_name("cs")
                .long("cs")
                .takes_value(true)
                .help("Specify chip-select GPIO number"),
        )
        .get_matches();
    let spi = matches.value_of("spi").ok_or("SPI must be specified")?;
    let cs = matches.value_of("cs").ok_or("CS must be specified")?;
    Ok((spi.into(), cs.parse().map_err(|_| "CS not a number")?))
}

struct AsyncSPI(Spidev);

#[async_trait::async_trait]
impl spi::AsyncSPI for AsyncSPI {
    type Error = io::Error;

    async fn trx(&mut self, tx: &[u8], rx: &mut [u8]) -> io::Result<()> {
        let mut transfer = match (tx.len(), rx.len()) {
            (0, _) => SpidevTransfer::read(rx),
            (_, 0) => SpidevTransfer::write(tx),
            (_, _) => SpidevTransfer::read_write(tx, rx),
        };
        self.0.transfer(&mut transfer)
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
        .max_speed_hz(400_000)
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

impl AsyncDelay for Delay {
    type Future = future::Ready<()>;

    fn delay(&mut self, duration: time::Duration) -> Self::Future {
        thread::sleep(duration);
        future::ready(())
    }
}

fn run() -> Result<(), String> {
    let (spi, cs) = parse_args()?;
    let spi = spidev(&spi).map_err(|e| e.to_string())?;
    let cs = gpio::sysfs::SysFsGpioOutput::open(cs).map_err(|e| e.to_string())?;
    let mut spi = spi::Bus::new(AsyncSPI(spi), GPIO(cs), CountDown::default());
    let card = task::block_on(spi.init(Delay)).map_err(|e| format!("{:?}", e))?;
    println!("Card: {:?}", card);
    let mut sd = task::block_on(SD::init(spi, card)).map_err(|e| format!("{:?}", e))?;
    println!("num-blocks {}", sd.num_blocks());

    let mut buffer = [0u8; 512];
    task::block_on(sd.read(0, &mut buffer)).map_err(|e| format!("{:?}", e))?;
    let mbr = MasterBootRecord::from_bytes(&buffer).map_err(|e| format!("{:?}", e))?;
    for partition in mbr.partition_table_entries().iter() {
        println!("{:?}", partition);
    }
    Ok(())
}

fn main() {
    log::set_max_level(log::LevelFilter::Debug);
    simple_log::console("debug").ok();
    match run() {
        Ok(_) => (),
        Err(e) => println!("{}", e),
    };
}
