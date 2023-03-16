pub trait Delay<UXX> {
    async fn delay_ms(&mut self, duration: UXX) -> ();
}

#[cfg(feature = "std")]
pub mod std {
    pub struct Delay;

    impl<UXX: Into<u64>> super::Delay<UXX> for Delay {
        async fn delay_ms(&mut self, ms: UXX) {
            std::thread::sleep(std::time::Duration::from_millis(ms.into()));
        }
    }

}
