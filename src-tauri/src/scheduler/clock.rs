//! Clock abstraction so scheduling logic is deterministic under test.

use chrono::{DateTime, Utc};

/// Source of "now". The production clock uses `Utc::now()`; tests inject a
/// controllable clock.
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

/// Real system clock.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

#[cfg(test)]
pub(crate) mod test_clock {
    use super::*;
    use std::sync::atomic::{AtomicI64, Ordering};

    /// Controllable clock stepping in whole seconds.
    #[derive(Debug, Default)]
    #[allow(dead_code)] // set/advance used by future virtual-time engine tests
    pub struct TestClock(pub AtomicI64);

    impl TestClock {
        #[allow(dead_code)] // exercised by future virtual-time engine tests
        pub fn at(secs: i64) -> std::sync::Arc<Self> {
            std::sync::Arc::new(Self(AtomicI64::new(secs)))
        }
        #[allow(dead_code)] // exercised by future virtual-time engine tests
        pub fn set(&self, secs: i64) {
            self.0.store(secs, Ordering::SeqCst);
        }
        #[allow(dead_code)] // exercised by future virtual-time engine tests
        pub fn advance(&self, secs: i64) {
            self.0.fetch_add(secs, Ordering::SeqCst);
        }
    }

    impl Clock for TestClock {
        fn now(&self) -> DateTime<Utc> {
            chrono::TimeZone::timestamp_opt(&Utc, self.0.load(Ordering::SeqCst), 0)
                .single()
                .expect("test clock timestamp")
        }
    }
}
