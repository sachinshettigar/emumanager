//! Real [`Clock`] backed by the system clock.

use emu_core::ports::Clock;
use time::OffsetDateTime;

/// The actual wall clock. Stateless — `emu-core`'s `FakeClock` is what tests use instead.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_is_close_to_the_real_time() {
        let before = OffsetDateTime::now_utc();
        let got = SystemClock.now();
        let after = OffsetDateTime::now_utc();
        assert!(got >= before && got <= after);
    }
}
