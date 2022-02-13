#[cfg(feature = "async")]
use core::future;

#[cfg(not(feature = "async"))]
use embedded_hal::blocking::delay::DelayMs;

#[cfg(feature = "async")]
pub trait Delay<UXX> {
    type Future: future::Future<Output = ()>;
    fn delay_ms(&mut self, duration: UXX) -> Self::Future;
}

#[cfg(not(feature = "async"))]
pub trait Delay<UXX>: DelayMs<UXX> {}
