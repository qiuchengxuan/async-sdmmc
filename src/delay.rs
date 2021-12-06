use core::future;
use core::time;

pub trait AsyncDelay {
    type Future: future::Future<Output = ()>;
    fn delay(&mut self, duration: time::Duration) -> Self::Future;
}
