use core::sync::atomic::{AtomicU64, Ordering};

use super::{Duration, UnixInstant};

/// An atomic precise measurement of the system clock.
///
/// Internally, it reprsents the instant as a whole number of nanoseconds from
/// the UNIX epoch using an `AtomicU64`. This provides interior mutability with
/// atomic operations.
///
/// See the [`crate::precise::UnixInstant`] type for more details.
#[repr(transparent)]
#[derive(Default, Debug)]
pub struct AtomicUnixInstant {
    ns: AtomicU64,
}

impl AtomicUnixInstant {
    /// Create a new `AtomicUnixInstant` representing the provided `UnixInstant`.
    pub fn new(value: UnixInstant) -> Self {
        Self {
            ns: value.ns.into(),
        }
    }

    /// Create a new `AtomicUnixInstant` representing the current instant.
    pub fn now() -> Self {
        Self::new(UnixInstant::now())
    }

    // Loads the value of the instant.
    ///
    /// See: [`core::sync::atomic::AtomicU64::load`] for a description of the
    /// memory orderings.
    ///
    /// # Panics
    /// Panics if `ordering` is `Release` or `AcqRel`.
    pub fn load(&self, ordering: Ordering) -> UnixInstant {
        UnixInstant {
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
    pub fn store(&self, value: UnixInstant, ordering: Ordering) {
        self.ns.store(value.ns, ordering)
    }

    /// Replaces the value of the instant and returns the previous value.
    ///
    /// See: [`core::sync::atomic::AtomicU64::swap`] for a description of the
    /// memory orderings.
    ///
    /// *Note*: This method is only available on platforms that support atomic
    /// operations on `u64`.
    pub fn swap(&self, value: UnixInstant, ordering: Ordering) -> UnixInstant {
        UnixInstant {
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
    pub fn fetch_add(&self, value: Duration, ordering: Ordering) -> UnixInstant {
        UnixInstant {
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
    pub fn fetch_max(&self, value: UnixInstant, ordering: Ordering) -> UnixInstant {
        UnixInstant {
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
    pub fn fetch_min(&self, value: UnixInstant, ordering: Ordering) -> UnixInstant {
        UnixInstant {
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
    pub fn fetch_sub(&self, value: Duration, ordering: Ordering) -> UnixInstant {
        UnixInstant {
            ns: self.ns.fetch_sub(value.ns, ordering),
        }
    }
}

impl From<UnixInstant> for AtomicUnixInstant {
    fn from(other: UnixInstant) -> Self {
        AtomicUnixInstant {
            ns: other.ns.into(),
        }
    }
}

impl From<crate::coarse::UnixInstant> for AtomicUnixInstant {
    fn from(other: crate::coarse::UnixInstant) -> Self {
        Self {
            ns: (other.secs as u64 * super::Duration::NANOSECOND.as_nanos()).into(),
        }
    }
}

pub struct TryFromError {
    kind: TryFromErrorKind,
}

enum TryFromErrorKind {
    Overflow,
    BeforeEpoch,
}

impl TryFromError {
    const fn description(&self) -> &'static str {
        match self.kind {
            TryFromErrorKind::Overflow => "can not convert to UnixInstant: value is too big",
            TryFromErrorKind::BeforeEpoch => {
                "can not convert to UnixInstant: value is before unix epoch"
            }
        }
    }
}

impl core::fmt::Display for TryFromError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.description().fmt(f)
    }
}

impl TryFrom<std::time::SystemTime> for AtomicUnixInstant {
    type Error = TryFromError;

    fn try_from(other: std::time::SystemTime) -> Result<Self, Self::Error> {
        let other = other
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .map_err(|_| TryFromError {
                kind: TryFromErrorKind::BeforeEpoch,
            })?
            .as_nanos();
        if other > u64::MAX as u128 {
            Err(TryFromError {
                kind: TryFromErrorKind::Overflow,
            })
        } else {
            Ok(Self {
                ns: (other as u64).into(),
            })
        }
    }
}
