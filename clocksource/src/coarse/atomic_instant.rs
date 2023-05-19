use core::sync::atomic::{AtomicU32, Ordering};

use super::{Duration, Instant};

#[repr(transparent)]
#[derive(Default, Debug)]
pub struct AtomicInstant {
    secs: AtomicU32,
}

impl AtomicInstant {
    pub fn new(value: Instant) -> Self {
        Self {
            secs: value.secs.into(),
        }
    }

    pub fn now() -> Self {
        Self::new(Instant::now())
    }

    pub fn load(&self, ordering: Ordering) -> Instant {
        Instant {
            secs: self.secs.load(ordering),
        }
    }

    pub fn store(&self, value: Instant, ordering: Ordering) {
        self.secs.store(value.secs, ordering)
    }

    pub fn swap(&self, value: Instant, ordering: Ordering) -> Instant {
        Instant {
            secs: self.secs.swap(value.secs, ordering),
        }
    }

    pub fn compare_exchange(
        &self,
        current: Instant,
        new: Instant,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Instant, Instant> {
        self.secs
            .compare_exchange(current.secs, new.secs, success, failure)
            .map(|secs| Instant { secs })
            .map_err(|secs| Instant { secs })
    }

    pub fn compare_exchange_weak(
        &self,
        current: Instant,
        new: Instant,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Instant, Instant> {
        self.secs
            .compare_exchange_weak(current.secs, new.secs, success, failure)
            .map(|secs| Instant { secs })
            .map_err(|secs| Instant { secs })
    }

    pub fn fetch_add(&self, value: Duration, ordering: Ordering) -> Instant {
        Instant {
            secs: self.secs.fetch_add(value.secs, ordering),
        }
    }

    pub fn fetch_max(&self, value: Instant, ordering: Ordering) -> Instant {
        Instant {
            secs: self.secs.fetch_max(value.secs, ordering),
        }
    }

    pub fn fetch_min(&self, value: Instant, ordering: Ordering) -> Instant {
        Instant {
            secs: self.secs.fetch_min(value.secs, ordering),
        }
    }

    pub fn fetch_sub(&self, value: Duration, ordering: Ordering) -> Instant {
        Instant {
            secs: self.secs.fetch_sub(value.secs, ordering),
        }
    }
}

impl From<Instant> for AtomicInstant {
    fn from(other: Instant) -> Self {
        AtomicInstant {
            secs: other.secs.into()
        }
    }
}

