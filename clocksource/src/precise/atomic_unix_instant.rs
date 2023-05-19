use core::sync::atomic::{AtomicU64, Ordering};

use super::{Duration, UnixInstant};

#[repr(transparent)]
#[derive(Default, Debug)]
pub struct AtomicUnixInstant {
    ns: AtomicU64,
}

impl AtomicUnixInstant {
    pub fn new(value: UnixInstant) -> Self {
        Self {
            ns: value.ns.into(),
        }
    }

    pub fn now() -> Self {
        Self::new(UnixInstant::now())
    }

    pub fn load(&self, ordering: Ordering) -> UnixInstant {
        UnixInstant {
            ns: self.ns.load(ordering),
        }
    }

    pub fn store(&self, value: UnixInstant, ordering: Ordering) {
        self.ns.store(value.ns, ordering)
    }

    pub fn swap(&self, value: UnixInstant, ordering: Ordering) -> UnixInstant {
        UnixInstant {
            ns: self.ns.swap(value.ns, ordering),
        }
    }

    pub fn compare_exchange(
        &self,
        current: UnixInstant,
        new: UnixInstant,
        success: Ordering,
        failure: Ordering,
    ) -> Result<UnixInstant, UnixInstant> {
        self.ns
            .compare_exchange(current.ns, new.ns, success, failure)
            .map(|ns| UnixInstant { ns })
            .map_err(|ns| UnixInstant { ns })
    }

    pub fn compare_exchange_weak(
        &self,
        current: UnixInstant,
        new: UnixInstant,
        success: Ordering,
        failure: Ordering,
    ) -> Result<UnixInstant, UnixInstant> {
        self.ns
            .compare_exchange_weak(current.ns, new.ns, success, failure)
            .map(|ns| UnixInstant { ns })
            .map_err(|ns| UnixInstant { ns })
    }

    pub fn fetch_add(&self, value: Duration, ordering: Ordering) -> UnixInstant {
        UnixInstant {
            ns: self.ns.fetch_add(value.ns, ordering),
        }
    }

    pub fn fetch_max(&self, value: UnixInstant, ordering: Ordering) -> UnixInstant {
        UnixInstant {
            ns: self.ns.fetch_max(value.ns, ordering),
        }
    }

    pub fn fetch_min(&self, value: UnixInstant, ordering: Ordering) -> UnixInstant {
        UnixInstant {
            ns: self.ns.fetch_min(value.ns, ordering),
        }
    }

    pub fn fetch_sub(&self, value: Duration, ordering: Ordering) -> UnixInstant {
        UnixInstant {
            ns: self.ns.fetch_sub(value.ns, ordering),
        }
    }
}
