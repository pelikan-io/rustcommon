use core::sync::atomic::{AtomicU64, Ordering};

use super::{Duration, Instant};

#[repr(transparent)]
#[derive(Default, Debug)]
pub struct AtomicInstant {
    ns: AtomicU64,
}

impl AtomicInstant {
    pub fn new(value: Instant) -> Self {
        Self {
            ns: value.ns.into(),
        }
    }

    pub fn now() -> Self {
        Self::new(Instant::now())
    }

    pub fn load(&self, ordering: Ordering) -> Instant {
        Instant {
            ns: self.ns.load(ordering),
        }
    }

    pub fn store(&self, value: Instant, ordering: Ordering) {
        self.ns.store(value.ns, ordering)
    }

    pub fn swap(&self, value: Instant, ordering: Ordering) -> Instant {
        Instant {
            ns: self.ns.swap(value.ns, ordering),
        }
    }

    pub fn compare_exchange(
        &self,
        current: Instant,
        new: Instant,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Instant, Instant> {
        self.ns
            .compare_exchange(current.ns, new.ns, success, failure)
            .map(|ns| Instant { ns })
            .map_err(|ns| Instant { ns })
    }

    pub fn compare_exchange_weak(
        &self,
        current: Instant,
        new: Instant,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Instant, Instant> {
        self.ns
            .compare_exchange_weak(current.ns, new.ns, success, failure)
            .map(|ns| Instant { ns })
            .map_err(|ns| Instant { ns })
    }

    pub fn fetch_add(&self, value: Duration, ordering: Ordering) -> Instant {
        Instant {
            ns: self.ns.fetch_add(value.ns, ordering),
        }
    }

    pub fn fetch_max(&self, value: Instant, ordering: Ordering) -> Instant {
        Instant {
            ns: self.ns.fetch_max(value.ns, ordering),
        }
    }

    pub fn fetch_min(&self, value: Instant, ordering: Ordering) -> Instant {
        Instant {
            ns: self.ns.fetch_min(value.ns, ordering),
        }
    }

    pub fn fetch_sub(&self, value: Duration, ordering: Ordering) -> Instant {
        Instant {
            ns: self.ns.fetch_sub(value.ns, ordering),
        }
    }
}

impl From<Instant> for AtomicInstant {
    fn from(other: Instant) -> Self {
        AtomicInstant {
            ns: other.ns.into()
        }
    }
}
