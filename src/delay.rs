#[cfg(feature = "async")]
use core::future;

#[cfg(not(feature = "async"))]
use embedded_hal::delay::DelayNs;

#[cfg(not(feature = "async"))]
pub trait Delay: DelayNs {}

#[cfg(not(feature = "async"))]
impl<T: DelayNs> Delay for T {}

#[cfg(feature = "async")]
pub trait Delay {
    type Future: future::Future<Output = ()>;
    fn delay_ms(&mut self, ms: u32) -> Self::Future;
}

#[cfg(feature = "std")]
pub mod std {
    pub struct Delay;

    #[cfg(feature = "async")]
    impl super::Delay for Delay {
        type Future = std::future::Ready<()>;

        fn delay_ms(&mut self, ms: u32) -> Self::Future {
            std::thread::sleep(std::time::Duration::from_millis(ms as u64));
            std::future::ready(())
        }
    }

    #[cfg(not(feature = "async"))]
    impl embedded_hal::delay::DelayNs for Delay {
        fn delay_ns(&mut self, ns: u32) {
            std::thread::sleep(std::time::Duration::from_nanos(ns as u64));
        }
    }
}
