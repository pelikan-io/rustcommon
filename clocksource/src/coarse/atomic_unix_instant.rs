use core::sync::atomic::{AtomicU32, Ordering};

use super::{Duration, UnixInstant};

#[repr(transparent)]
#[derive(Default, Debug)]
pub struct AtomicUnixInstant {
    secs: AtomicU32,
}

impl AtomicUnixInstant {
    pub fn new(value: UnixInstant) -> Self {
        Self {
            secs: value.secs.into(),
        }
    }

    pub fn now() -> Self {
        Self::new(UnixInstant::now())
    }

    pub fn load(&self, ordering: Ordering) -> UnixInstant {
        UnixInstant {
            secs: self.secs.load(ordering),
        }
    }

    pub fn store(&self, value: UnixInstant, ordering: Ordering) {
        self.secs.store(value.secs, ordering)
    }

    pub fn swap(&self, value: UnixInstant, ordering: Ordering) -> UnixInstant {
        UnixInstant {
            secs: self.secs.swap(value.secs, ordering),
        }
    }

    pub fn compare_exchange(
        &self,
        current: UnixInstant,
        new: UnixInstant,
        success: Ordering,
        failure: Ordering,
    ) -> Result<UnixInstant, UnixInstant> {
        self.secs
            .compare_exchange(current.secs, new.secs, success, failure)
            .map(|secs| UnixInstant { secs })
            .map_err(|secs| UnixInstant { secs })
    }

    pub fn compare_exchange_weak(
        &self,
        current: UnixInstant,
        new: UnixInstant,
        success: Ordering,
        failure: Ordering,
    ) -> Result<UnixInstant, UnixInstant> {
        self.secs
            .compare_exchange_weak(current.secs, new.secs, success, failure)
            .map(|secs| UnixInstant { secs })
            .map_err(|secs| UnixInstant { secs })
    }

    pub fn fetch_add(&self, value: Duration, ordering: Ordering) -> UnixInstant {
        UnixInstant {
            secs: self.secs.fetch_add(value.secs, ordering),
        }
    }

    pub fn fetch_max(&self, value: UnixInstant, ordering: Ordering) -> UnixInstant {
        UnixInstant {
            secs: self.secs.fetch_max(value.secs, ordering),
        }
    }

    pub fn fetch_min(&self, value: UnixInstant, ordering: Ordering) -> UnixInstant {
        UnixInstant {
            secs: self.secs.fetch_min(value.secs, ordering),
        }
    }

    pub fn fetch_sub(&self, value: Duration, ordering: Ordering) -> UnixInstant {
        UnixInstant {
            secs: self.secs.fetch_sub(value.secs, ordering),
        }
    }
}
