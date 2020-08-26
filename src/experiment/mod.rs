pub mod analyser;
pub mod receiver;
pub mod results;
pub mod sender;
pub mod statement;
pub mod visualiser;

pub trait SmallishDuration {
    /// as_micros is busy
    fn as_us(&self) -> u32;
}
impl SmallishDuration for ::std::time::Duration {
    fn as_us(&self) -> u32 {
        self.subsec_micros() + 1000_000u32 * (self.as_secs() as u32)
    }
}
