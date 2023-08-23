use core::sync::atomic::{AtomicU32, Ordering};

use super::{Duration, Instant};

/// A `coarse::AtomicInstant` is a measurement of a monotonically
/// nondecreasing clock. It is opaque and useful only with the duration types.
///
/// Internally, it reprsents the instant as a whole number of seconds from an
/// arbitrary epoch using an `AtomicU32`. This provides interior mutability with
/// atomic operations.
///
/// See the [`crate::coarse::Instant`] type for more details.
#[repr(transparent)]
#[derive(Default, Debug)]
pub struct AtomicInstant {
    secs: AtomicU32,
}

impl AtomicInstant {
    /// Create a new `AtomicInstant` representing the provided `Instant`.
    pub fn new(value: Instant) -> Self {
        Self {
            secs: value.secs.into(),
        }
    }

    /// Create a new `AtomicInstant` representing the current instant.
    pub fn now() -> Self {
        Self::new(Instant::now())
    }

    // Loads the value of the instant.
    ///
    /// See: [`core::sync::atomic::AtomicU32::load`] for a description of the
    /// memory orderings.
    ///
    /// # Panics
    /// Panics if `ordering` is `Release` or `AcqRel`.
    pub fn load(&self, ordering: Ordering) -> Instant {
        Instant {
            secs: self.secs.load(ordering),
        }
    }

    /// Stores a new value for the instant.
    ///
    /// See: [`core::sync::atomic::AtomicU32::store`] for a description of the
    /// memory orderings.
    ///
    /// # Panics
    /// Panics if `ordering` is `Acquire` or `AcqRel`.
    pub fn store(&self, value: Instant, ordering: Ordering) {
        self.secs.store(value.secs, ordering)
    }

    /// Replaces the value of the instant and returns the previous value.
    ///
    /// See: [`core::sync::atomic::AtomicU32::swap`] for a description of the
    /// memory orderings.
    ///
    /// *Note*: This method is only available on platforms that support atomic
    /// operations on `u32`.
    pub fn swap(&self, value: Instant, ordering: Ordering) -> Instant {
        Instant {
            secs: self.secs.swap(value.secs, ordering),
        }
    }

    /// Stores a new value for the instant if the current instant is the same as
    /// the `current` instant.
    ///
    /// See: [`core::sync::atomic::AtomicU32::compare_exchange`] for a
    /// description of the memory orderings.
    ///
    /// *Note*: This method is only available on platforms that support atomic
    /// operations on `u32`.
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

    /// Stores a new value for the instant if the current instant is the same as
    /// the `current` instant.
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

    /// Adds to the current instant, returning the previous instant.
    ///
    /// This operation wraps around on overflow.
    ///
    /// See: [`core::sync::atomic::AtomicU32::fetch_add`] for a
    /// description of the memory orderings.
    ///
    /// *Note*: This method is only available on platforms that support atomic
    /// operations on `u32`.
    pub fn fetch_add(&self, value: Duration, ordering: Ordering) -> Instant {
        Instant {
            secs: self.secs.fetch_add(value.secs, ordering),
        }
    }

    /// Maximum with the current instant.
    ///
    /// Finds the maximum of the current instant and the argument `value`, and
    /// sets the new instant to the result.
    ///
    /// Returns the previous instant.
    ///
    /// See: [`core::sync::atomic::AtomicU32::fetch_max`] for a
    /// description of the memory orderings.
    ///
    /// *Note*: This method is only available on platforms that support atomic
    /// operations on `u32`.
    pub fn fetch_max(&self, value: Instant, ordering: Ordering) -> Instant {
        Instant {
            secs: self.secs.fetch_max(value.secs, ordering),
        }
    }

    /// Minimum with the current instant.
    ///
    /// Finds the minimum of the current instant and the argument `val`, and
    /// sets the new instant to the result.
    ///
    /// Returns the previous instant.
    ///
    /// See: [`core::sync::atomic::AtomicU32::fetch_min`] for a
    /// description of the memory orderings.
    ///
    /// *Note*: This method is only available on platforms that support atomic
    /// operations on `u32`.
    pub fn fetch_min(&self, value: Instant, ordering: Ordering) -> Instant {
        Instant {
            secs: self.secs.fetch_min(value.secs, ordering),
        }
    }

    /// Subtracts from the current instant, returning the previous instant.
    ///
    /// This operation wraps around on overflow.
    ///
    /// See: [`core::sync::atomic::AtomicU32::fetch_sub`] for a
    /// description of the memory orderings.
    ///
    /// *Note*: This method is only available on platforms that support atomic
    /// operations on `u32`.
    pub fn fetch_sub(&self, value: Duration, ordering: Ordering) -> Instant {
        Instant {
            secs: self.secs.fetch_sub(value.secs, ordering),
        }
    }
}

impl From<Instant> for AtomicInstant {
    fn from(other: Instant) -> Self {
        AtomicInstant {
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
            TryFromErrorKind::Overflow => "can not convert to UnixInstant: value is too big",
        }
    }
}

impl core::fmt::Display for TryFromError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.description().fmt(f)
    }
}

impl TryFrom<crate::precise::Instant> for AtomicInstant {
    type Error = TryFromError;

    fn try_from(other: crate::precise::Instant) -> Result<Self, Self::Error> {
        let other = other.ns / crate::precise::Duration::SECOND.as_nanos();
        if other > u32::MAX as u64 {
            Err(TryFromError {
                kind: TryFromErrorKind::Overflow,
            })
        } else {
            Ok(Self {
                secs: (other as u32).into(),
            })
        }
    }
}
