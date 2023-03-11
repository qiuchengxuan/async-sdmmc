sdmmc
=====

> A sdmmc implementation mainly focusing on embedded system with `no_std` and `async` support

Using this crate
----------------

Assuming you already have `SPI` struct which implements `sdmmc::spi::Transfer`

```rust
let mut bus = sdmmc::bus::linux::spi(&args.spi, args.cs).map_err(|e| format!("{:?}", e))?;
let card = bus.init(Delay).await.map_err(|e| format!("{:?}", e))?;
debug!("Card: {:?}", card);
let mut sd = SD::init(bus, card).await.map_err(|e| format!("{:?}", e))?;
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
```

Features
--------

* **async**

  Enable async support

* **async-trait**

  Use async-trait, otherwise nightly unstable `async_fn_in_trait` feature is enabled

* **std**

  Use std library

* **linux-spi**

  Enable linux SPI support

* **log-max-level-off**

  Disable logging at compile time
