use core::sync::atomic::{AtomicU64, Ordering};

use super::Duration;

#[repr(transparent)]
#[derive(Default, Debug)]
pub struct AtomicDuration {
    ns: AtomicU64,
}

impl AtomicDuration {
    pub fn new(value: Duration) -> Self {
        value.into()
    }

    pub fn from_secs(secs: u32) -> Self {
        Duration::from_secs(secs).into()
    }

    pub fn from_nanos(nanos: u64) -> Self {
        Duration::from_nanos(nanos).into()
    }

    pub fn load(&self, ordering: Ordering) -> Duration {
        Duration {
            ns: self.ns.load(ordering),
        }
    }

    pub fn store(&self, value: Duration, ordering: Ordering) {
        self.ns.store(value.ns, ordering)
    }

    pub fn swap(&self, value: Duration, ordering: Ordering) -> Duration {
        Duration {
            ns: self.ns.swap(value.ns, ordering),
        }
    }

    pub fn compare_exchange(
        &self,
        current: Duration,
        new: Duration,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Duration, Duration> {
        self.ns
            .compare_exchange(current.ns, new.ns, success, failure)
            .map(|ns| Duration { ns })
            .map_err(|ns| Duration { ns })
    }

    pub fn compare_exchange_weak(
        &self,
        current: Duration,
        new: Duration,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Duration, Duration> {
        self.ns
            .compare_exchange_weak(current.ns, new.ns, success, failure)
            .map(|ns| Duration { ns })
            .map_err(|ns| Duration { ns })
    }

    pub fn fetch_add(&self, value: Duration, ordering: Ordering) -> Duration {
        Duration {
            ns: self.ns.fetch_add(value.ns, ordering),
        }
    }

    pub fn fetch_max(&self, value: Duration, ordering: Ordering) -> Duration {
        Duration {
            ns: self.ns.fetch_max(value.ns, ordering),
        }
    }

    pub fn fetch_min(&self, value: Duration, ordering: Ordering) -> Duration {
        Duration {
            ns: self.ns.fetch_min(value.ns, ordering),
        }
    }

    pub fn fetch_sub(&self, value: Duration, ordering: Ordering) -> Duration {
        Duration {
            ns: self.ns.fetch_sub(value.ns, ordering),
        }
    }
}

impl From<Duration> for AtomicDuration {
    fn from(other: Duration) -> Self {
        Self {
            ns: other.ns.into(),
        }
    }
}
