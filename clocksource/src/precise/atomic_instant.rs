use core::sync::atomic::{AtomicU64, Ordering};

use super::{Duration, Instant};

/// An atomic precise measurement of a monotonically nodecreasing clock.
///
/// It is opaque and useful only with the duration types.
///
/// Internally, it reprsents the instant as a whole number of nanoseconds from
/// an arbitrary epoch using an `AtomicU64`. This provides interior mutability
/// with atomic operations.
///
/// See the [`crate::precise::Instant`] type for more details.
#[repr(transparent)]
#[derive(Default, Debug)]
pub struct AtomicInstant {
    ns: AtomicU64,
}

impl AtomicInstant {
    /// Create a new `AtomicInstant` representing the provided `Instant`.
    pub fn new(value: Instant) -> Self {
        Self {
            ns: value.ns.into(),
        }
    }

    /// Create a new `AtomicInstant` representing the current instant.
    pub fn now() -> Self {
        Self::new(Instant::now())
    }

    // Loads the value of the instant.
    ///
    /// See: [`core::sync::atomic::AtomicU64::load`] for a description of the
    /// memory orderings.
    ///
    /// # Panics
    /// Panics if `ordering` is `Release` or `AcqRel`.
    pub fn load(&self, ordering: Ordering) -> Instant {
        Instant {
            ns: self.ns.load(ordering),
        }
    }

    /// Stores a new value for the instant.
    ///
    /// See: [`core::sync::atomic::AtomicU64::store`] for a description of the
    /// memory orderings.
    ///
    /// # Panics
    /// Panics if `ordering` is `Acquire` or `AcqRel`.
    pub fn store(&self, value: Instant, ordering: Ordering) {
        self.ns.store(value.ns, ordering)
    }

    /// Replaces the value of the instant and returns the previous value.
    ///
    /// See: [`core::sync::atomic::AtomicU64::swap`] for a description of the
    /// memory orderings.
    ///
    /// *Note*: This method is only available on platforms that support atomic
    /// operations on `u64`.
    pub fn swap(&self, value: Instant, ordering: Ordering) -> Instant {
        Instant {
            ns: self.ns.swap(value.ns, ordering),
        }
    }

    /// Stores a new value for the instant if the current instant is the same as
    /// the `current` instant.
    ///
    /// See: [`core::sync::atomic::AtomicU64::compare_exchange`] for a
    /// description of the memory orderings.
    ///
    /// *Note*: This method is only available on platforms that support atomic
    /// operations on `u64`.
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

    /// Stores a new value for the instant if the current instant is the same as
    /// the `current` instant.
    ///
    /// See: [`core::sync::atomic::AtomicU64::compare_exchange`] for a
    /// description of the memory orderings.
    ///
    /// *Note*: This method is only available on platforms that support atomic
    /// operations on `u64`.
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

    /// Stores a new value for the instant if the current instant is the same as
    /// the `current` instant.
    ///
    /// See: [`core::sync::atomic::AtomicU64::compare_exchange_weak`] for a
    /// description of the memory orderings.
    ///
    /// Unlike `AtomicDuration::compare_exchange`, this function is allowed to
    /// spuriously fail. This allows for more efficient code on some platforms.
    ///
    /// *Note*: This method is only available on platforms that support atomic
    /// operations on `u64`.
    pub fn fetch_add(&self, value: Duration, ordering: Ordering) -> Instant {
        Instant {
            ns: self.ns.fetch_add(value.ns, ordering),
        }
    }

    /// Maximum with the current instant.
    ///
    /// Finds the maximum of the current instant and the argument `value`, and
    /// sets the new instant to the result.
    ///
    /// Returns the previous instant.
    ///
    /// See: [`core::sync::atomic::AtomicU64::fetch_max`] for a
    /// description of the memory orderings.
    ///
    /// *Note*: This method is only available on platforms that support atomic
    /// operations on `u64`.
    pub fn fetch_max(&self, value: Instant, ordering: Ordering) -> Instant {
        Instant {
            ns: self.ns.fetch_max(value.ns, ordering),
        }
    }

    /// Minimum with the current instant.
    ///
    /// Finds the minimum of the current instant and the argument `val`, and
    /// sets the new instant to the result.
    ///
    /// Returns the previous instant.
    ///
    /// See: [`core::sync::atomic::AtomicU64::fetch_min`] for a
    /// description of the memory orderings.
    ///
    /// *Note*: This method is only available on platforms that support atomic
    /// operations on `u64`.
    pub fn fetch_min(&self, value: Instant, ordering: Ordering) -> Instant {
        Instant {
            ns: self.ns.fetch_min(value.ns, ordering),
        }
    }

    /// Subtracts from the current instant, returning the previous instant.
    ///
    /// This operation wraps around on overflow.
    ///
    /// See: [`core::sync::atomic::AtomicU64::fetch_sub`] for a
    /// description of the memory orderings.
    ///
    /// *Note*: This method is only available on platforms that support atomic
    /// operations on `u64`.
    pub fn fetch_sub(&self, value: Duration, ordering: Ordering) -> Instant {
        Instant {
            ns: self.ns.fetch_sub(value.ns, ordering),
        }
    }
}

impl From<Instant> for AtomicInstant {
    fn from(other: Instant) -> Self {
        AtomicInstant {
            ns: other.ns.into(),
        }
    }
}

impl From<crate::coarse::Instant> for AtomicInstant {
    fn from(other: crate::coarse::Instant) -> Self {
        Self {
            ns: (other.secs as u64 * super::Duration::NANOSECOND.as_nanos()).into(),
        }
    }
}
