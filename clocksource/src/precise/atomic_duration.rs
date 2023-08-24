use core::sync::atomic::{AtomicU64, Ordering};

use super::Duration;

/// An atomic duration measured in nanoseconds.
///
/// A `precise::AtomicDuration` is a duration that is measured in nanoseconds
/// and represented as an unsigned 64bit value. Since it is implemented using
/// atomic primitives, it can be used when the duration needs interior
/// mutability and atomic operations.
#[repr(transparent)]
#[derive(Default, Debug)]
pub struct AtomicDuration {
    ns: AtomicU64,
}

impl AtomicDuration {
    /// Create a new atomic duration.
    pub fn new(value: Duration) -> Self {
        value.into()
    }

    /// Create a new atomic duration that represents the provided number of
    /// seconds.
    pub fn from_secs(secs: u32) -> Self {
        Duration::from_secs(secs).into()
    }

    /// Create a new atomic duration that represents the provided number of
    /// nanoseconds.
    pub fn from_nanos(nanos: u64) -> Self {
        Duration::from_nanos(nanos).into()
    }

    /// Loads the value of the duration.
    ///
    /// See: [`core::sync::atomic::AtomicU64::load`] for a description of the
    /// memory orderings.
    ///
    /// # Panics
    /// Panics if `ordering` is `Release` or `AcqRel`.
    pub fn load(&self, ordering: Ordering) -> Duration {
        Duration {
            ns: self.ns.load(ordering),
        }
    }

    /// Stores a new value for the duration.
    ///
    /// See: [`core::sync::atomic::AtomicU64::store`] for a description of the
    /// memory orderings.
    ///
    /// # Panics
    /// Panics if `ordering` is `Acquire` or `AcqRel`.
    pub fn store(&self, value: Duration, ordering: Ordering) {
        self.ns.store(value.ns, ordering)
    }

    /// Replaces the value of the duration and returns the previous value.
    ///
    /// See: [`core::sync::atomic::AtomicU64::swap`] for a description of the
    /// memory orderings.
    ///
    /// *Note*: This method is only available on platforms that support atomic
    /// operations on `u64`.
    pub fn swap(&self, value: Duration, ordering: Ordering) -> Duration {
        Duration {
            ns: self.ns.swap(value.ns, ordering),
        }
    }

    /// Stores a new value for the duration if the current duration is the same
    /// as the `current` duration.
    ///
    /// See: [`core::sync::atomic::AtomicU64::compare_exchange`] for a
    /// description of the memory orderings.
    ///
    /// *Note*: This method is only available on platforms that support atomic
    /// operations on `u64`.
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

    /// Stores a new value for the duration if the current duration is the same
    /// as the `current` duration.
    ///
    /// See: [`core::sync::atomic::AtomicU64::compare_exchange_weak`] for a
    /// description of the memory orderings.
    ///
    /// Unlike `AtomicDuration::compare_exchange`, this function is allowed to
    /// spuriously fail. This allows for more efficient code on some platforms.
    ///
    /// *Note*: This method is only available on platforms that support atomic
    /// operations on `u64`.
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

    /// Adds to the current duration, returning the previous duration.
    ///
    /// This operation wraps around on overflow.
    ///
    /// See: [`core::sync::atomic::AtomicU64::fetch_add`] for a
    /// description of the memory orderings.
    ///
    /// *Note*: This method is only available on platforms that support atomic
    /// operations on `u64`.
    pub fn fetch_add(&self, value: Duration, ordering: Ordering) -> Duration {
        Duration {
            ns: self.ns.fetch_add(value.ns, ordering),
        }
    }

    /// Maximum with the current duration.
    ///
    /// Finds the maximum of the current duration and the argument `value`, and
    /// sets the new duration to the result.
    ///
    /// Returns the previous duration.
    ///
    /// See: [`core::sync::atomic::AtomicU64::fetch_max`] for a
    /// description of the memory orderings.
    ///
    /// *Note*: This method is only available on platforms that support atomic
    /// operations on `u64`.
    pub fn fetch_max(&self, value: Duration, ordering: Ordering) -> Duration {
        Duration {
            ns: self.ns.fetch_max(value.ns, ordering),
        }
    }

    /// Minimum with the current duration.
    ///
    /// Finds the minimum of the current duration and the argument `val`, and
    /// sets the new duration to the result.
    ///
    /// Returns the previous duration.
    ///
    /// See: [`core::sync::atomic::AtomicU64::fetch_min`] for a
    /// description of the memory orderings.
    ///
    /// *Note*: This method is only available on platforms that support atomic
    /// operations on `u64`.
    pub fn fetch_min(&self, value: Duration, ordering: Ordering) -> Duration {
        Duration {
            ns: self.ns.fetch_min(value.ns, ordering),
        }
    }

    /// Subtracts from the current duration, returning the previous duration.
    ///
    /// This operation wraps around on overflow.
    ///
    /// See: [`core::sync::atomic::AtomicU64::fetch_sub`] for a
    /// description of the memory orderings.
    ///
    /// *Note*: This method is only available on platforms that support atomic
    /// operations on `u64`.
    pub fn fetch_sub(&self, value: Duration, ordering: Ordering) -> Duration {
        Duration {
            ns: self.ns.fetch_sub(value.ns, ordering),
        }
    }
}

impl From<Duration> for AtomicDuration {
    fn from(other: Duration) -> Self {
        Self::new(other)
    }
}

impl From<crate::coarse::Duration> for AtomicDuration {
    fn from(other: crate::coarse::Duration) -> Self {
        Self::new(other.into())
    }
}

pub struct TryFromError {
    kind: TryFromErrorKind,
}

enum TryFromErrorKind {
    Overflow,
}

impl TryFromError {
    const fn description(&self) -> &'static str {
        match self.kind {
            TryFromErrorKind::Overflow => "can not convert to Duration: value is too big",
        }
    }
}

impl core::fmt::Display for TryFromError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.description().fmt(f)
    }
}

impl TryFrom<core::time::Duration> for AtomicDuration {
    type Error = TryFromError;

    fn try_from(other: core::time::Duration) -> Result<Self, Self::Error> {
        if other.as_nanos() > u64::MAX as u128 {
            Err(TryFromError {
                kind: TryFromErrorKind::Overflow,
            })
        } else {
            Ok(Self::from_nanos(other.as_nanos() as u64))
        }
    }
}
