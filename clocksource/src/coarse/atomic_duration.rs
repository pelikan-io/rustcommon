use core::sync::atomic::{AtomicU32, Ordering};

use super::Duration;

#[repr(transparent)]
#[derive(Default, Debug)]
pub struct AtomicDuration {
    secs: AtomicU32,
}

impl AtomicDuration {
    pub fn new(value: Duration) -> Self {
        value.into()
    }

    pub fn from_secs(secs: u32) -> Self {
        Duration::from_secs(secs).into()
    }

    pub fn load(&self, ordering: Ordering) -> Duration {
        Duration {
            secs: self.secs.load(ordering),
        }
    }

    pub fn store(&self, value: Duration, ordering: Ordering) {
        self.secs.store(value.secs, ordering)
    }

    pub fn swap(&self, value: Duration, ordering: Ordering) -> Duration {
        Duration {
            secs: self.secs.swap(value.secs, ordering),
        }
    }

    pub fn compare_exchange(
        &self,
        current: Duration,
        new: Duration,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Duration, Duration> {
        self.secs
            .compare_exchange(current.secs, new.secs, success, failure)
            .map(|secs| Duration { secs })
            .map_err(|secs| Duration { secs })
    }

    pub fn compare_exchange_weak(
        &self,
        current: Duration,
        new: Duration,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Duration, Duration> {
        self.secs
            .compare_exchange_weak(current.secs, new.secs, success, failure)
            .map(|secs| Duration { secs })
            .map_err(|secs| Duration { secs })
    }

    pub fn fetch_add(&self, value: Duration, ordering: Ordering) -> Duration {
        Duration {
            secs: self.secs.fetch_add(value.secs, ordering),
        }
    }

    pub fn fetch_max(&self, value: Duration, ordering: Ordering) -> Duration {
        Duration {
            secs: self.secs.fetch_max(value.secs, ordering),
        }
    }

    pub fn fetch_min(&self, value: Duration, ordering: Ordering) -> Duration {
        Duration {
            secs: self.secs.fetch_min(value.secs, ordering),
        }
    }

    pub fn fetch_sub(&self, value: Duration, ordering: Ordering) -> Duration {
        Duration {
            secs: self.secs.fetch_sub(value.secs, ordering),
        }
    }
}

impl From<Duration> for AtomicDuration {
    fn from(other: Duration) -> Self {
        Self {
            secs: other.secs.into(),
        }
    }
}

