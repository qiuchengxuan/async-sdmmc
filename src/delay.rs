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

#[cfg(not(feature = "async"))]
impl<UXX, T: DelayMs<UXX>> Delay<UXX> for T {}

#[cfg(feature = "std")]
pub mod std {
    pub struct Delay;

    #[cfg(feature = "async")]
    impl<UXX: Into<u64>> super::Delay<UXX> for Delay {
        type Future = std::future::Ready<()>;

        fn delay_ms(&mut self, ms: UXX) -> Self::Future {
            std::thread::sleep(std::time::Duration::from_millis(ms.into()));
            std::future::ready(())
        }
    }

    #[cfg(not(feature = "async"))]
    impl<UXX: Into<u64>> embedded_hal::blocking::delay::DelayMs<UXX> for Delay {
        fn delay_ms(&mut self, ms: UXX) {
            std::thread::sleep(std::time::Duration::from_millis(ms.into()));
        }
    }
}
