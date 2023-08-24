use core::sync::atomic::{AtomicU32, Ordering};

use super::Duration;

/// A `coarse::AtomicDuration` is a duration that is measured in seconds and
/// represented as an unsigned 32bit value. Since it is implemented using atomic
/// primitives, it can be used when the duration needs interior mutability and
/// atomic operations.
#[repr(transparent)]
#[derive(Default, Debug)]
pub struct AtomicDuration {
    secs: AtomicU32,
}

impl AtomicDuration {
    /// Create a new atomic duration.
    pub fn new(value: Duration) -> Self {
        value.into()
    }

    /// Create a new atomic duration from a whole number of seconds.
    pub fn from_secs(secs: u32) -> Self {
        Duration::from_secs(secs).into()
    }

    /// Loads the value of the duration.
    ///
    /// See: [`core::sync::atomic::AtomicU32::load`] for a description of the
    /// memory orderings.
    ///
    /// # Panics
    /// Panics if `ordering` is `Release` or `AcqRel`.
    pub fn load(&self, ordering: Ordering) -> Duration {
        Duration {
            secs: self.secs.load(ordering),
        }
    }

    /// Stores a new value for the duration.
    ///
    /// See: [`core::sync::atomic::AtomicU32::store`] for a description of the
    /// memory orderings.
    ///
    /// # Panics
    /// Panics if `ordering` is `Acquire` or `AcqRel`.
    pub fn store(&self, value: Duration, ordering: Ordering) {
        self.secs.store(value.secs, ordering)
    }

    /// Replaces the value of the duration and returns the previous value.
    ///
    /// See: [`core::sync::atomic::AtomicU32::swap`] for a description of the
    /// memory orderings.
    ///
    /// *Note*: This method is only available on platforms that support atomic
    /// operations on `u32`.
    pub fn swap(&self, value: Duration, order: Ordering) -> Duration {
        Duration {
            secs: self.secs.swap(value.secs, order),
        }
    }

    /// Stores a new value for the duration if the current duration is the same
    /// as the `current` duration.
    ///
    /// See: [`core::sync::atomic::AtomicU32::compare_exchange`] for a
    /// description of the memory orderings.
    ///
    /// *Note*: This method is only available on platforms that support atomic
    /// operations on `u32`.
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

    /// Stores a new value for the duration if the current duration is the same
    /// as the `current` duration.
    ///
    /// See: [`core::sync::atomic::AtomicU32::compare_exchange_weak`] for a
    /// description of the memory orderings.
    ///
    /// Unlike `AtomicDuration::compare_exchange`, this function is allowed to
    /// spuriously fail. This allows for more efficient code on some platforms.
    ///
    /// *Note*: This method is only available on platforms that support atomic
    /// operations on `u32`.
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

    /// Adds to the current duration, returning the previous duration.
    ///
    /// This operation wraps around on overflow.
    ///
    /// See: [`core::sync::atomic::AtomicU32::fetch_add`] for a
    /// description of the memory orderings.
    ///
    /// *Note*: This method is only available on platforms that support atomic
    /// operations on `u32`.
    pub fn fetch_add(&self, value: Duration, ordering: Ordering) -> Duration {
        Duration {
            secs: self.secs.fetch_add(value.secs, ordering),
        }
    }

    /// Maximum with the current duration.
    ///
    /// Finds the maximum of the current duration and the argument `value`, and
    /// sets the new duration to the result.
    ///
    /// Returns the previous duration.
    ///
    /// See: [`core::sync::atomic::AtomicU32::fetch_max`] for a
    /// description of the memory orderings.
    ///
    /// *Note*: This method is only available on platforms that support atomic
    /// operations on `u32`.
    pub fn fetch_max(&self, value: Duration, ordering: Ordering) -> Duration {
        Duration {
            secs: self.secs.fetch_max(value.secs, ordering),
        }
    }

    /// Minimum with the current duration.
    ///
    /// Finds the minimum of the current duration and the argument `val`, and
    /// sets the new duration to the result.
    ///
    /// Returns the previous duration.
    ///
    /// See: [`core::sync::atomic::AtomicU32::fetch_min`] for a
    /// description of the memory orderings.
    ///
    /// *Note*: This method is only available on platforms that support atomic
    /// operations on `u32`.
    pub fn fetch_min(&self, val: Duration, ordering: Ordering) -> Duration {
        Duration {
            secs: self.secs.fetch_min(val.secs, ordering),
        }
    }

    /// Subtracts from the current duration, returning the previous duration.
    ///
    /// This operation wraps around on overflow.
    ///
    /// See: [`core::sync::atomic::AtomicU32::fetch_sub`] for a
    /// description of the memory orderings.
    ///
    /// *Note*: This method is only available on platforms that support atomic
    /// operations on `u32`.
    pub fn fetch_sub(&self, val: Duration, ordering: Ordering) -> Duration {
        Duration {
            secs: self.secs.fetch_sub(val.secs, ordering),
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
        if other.as_secs() > u32::MAX as u64 {
            Err(TryFromError {
                kind: TryFromErrorKind::Overflow,
            })
        } else {
            Ok(Self::from_secs(other.as_secs() as u32))
        }
    }
}
