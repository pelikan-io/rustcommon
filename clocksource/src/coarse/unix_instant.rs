use core::ops::{Add, AddAssign, Sub, SubAssign};

use super::Duration;

/// A measurement of the system clock in seconds.
///
/// An `precise::UnixInstant` represents a moment in time and is taken from the
/// system realtime clock. Unlike `std::time::SystemTime` the internal
/// representation uses only seconds in a u32 field to hold the clock reading.
///
/// This will wrap on Feb 07 2106 (UTC) and cannot represent times before the
/// UNIX epoch on Jan 01 1970 (UTC).
///
/// As with `std::time::SystemTime`, `UnixInstant`s are not guaranteed to be
/// steady. They are taken from a clock which is subject to phase and frequency
/// adjustments. This means that they may jump forward or backwards and speed up
/// or slow down.
///
/// This type is useful for representing moments in time across restarts and
/// across systems as long as the clocks are reasonably synchronized to a common
/// reference.
///
/// The size of a `coarse::UnixInstant` is always the same as a `u32`.
#[repr(transparent)]
#[derive(Copy, Clone, Default, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnixInstant {
    pub(crate) secs: u32,
}

impl UnixInstant {
    pub const EPOCH: UnixInstant = UnixInstant { secs: 0 };

    /// Return a `UnixInstant` that represents the current moment.
    pub fn now() -> Self {
        crate::sys::realtime::coarse()
    }

    /// Return the elapsed time, in nanoseconds, since the original timestamp.
    pub fn elapsed(&self) -> Duration {
        Self::now() - *self
    }

    /// Return the elapsed duration, in nanoseconds, from some earlier timestamp
    /// until this timestamp.
    pub fn duration_since(&self, earlier: Self) -> Duration {
        *self - earlier
    }

    pub fn checked_duration_since(&self, earlier: Self) -> Option<Duration> {
        self.secs
            .checked_sub(earlier.secs)
            .map(|secs| Duration { secs })
    }

    pub fn checked_sub(&self, duration: Duration) -> Option<Self> {
        self.secs
            .checked_sub(duration.secs)
            .map(|secs| Self { secs })
    }
}

impl Add<Duration> for UnixInstant {
    type Output = UnixInstant;

    fn add(self, rhs: Duration) -> Self::Output {
        UnixInstant {
            secs: self.secs + rhs.secs,
        }
    }
}

impl Add<core::time::Duration> for UnixInstant {
    type Output = UnixInstant;

    fn add(self, rhs: core::time::Duration) -> Self::Output {
        UnixInstant {
            secs: self.secs + rhs.as_secs() as u32,
        }
    }
}

impl Sub<UnixInstant> for UnixInstant {
    type Output = Duration;

    fn sub(self, rhs: UnixInstant) -> Self::Output {
        Duration {
            secs: self.secs - rhs.secs,
        }
    }
}

impl AddAssign<Duration> for UnixInstant {
    fn add_assign(&mut self, rhs: Duration) {
        self.secs += rhs.secs;
    }
}

impl Sub<Duration> for UnixInstant {
    type Output = UnixInstant;

    fn sub(self, rhs: Duration) -> Self::Output {
        UnixInstant {
            secs: self.secs - rhs.secs,
        }
    }
}

impl SubAssign<Duration> for UnixInstant {
    fn sub_assign(&mut self, rhs: Duration) {
        self.secs -= rhs.secs;
    }
}

impl AddAssign<core::time::Duration> for UnixInstant {
    fn add_assign(&mut self, rhs: core::time::Duration) {
        self.secs += rhs.as_secs() as u32;
    }
}

impl Sub<core::time::Duration> for UnixInstant {
    type Output = UnixInstant;

    fn sub(self, rhs: core::time::Duration) -> Self::Output {
        UnixInstant {
            secs: self.secs - rhs.as_secs() as u32,
        }
    }
}

impl SubAssign<core::time::Duration> for UnixInstant {
    fn sub_assign(&mut self, rhs: core::time::Duration) {
        self.secs -= rhs.as_secs() as u32;
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

impl TryFrom<std::time::SystemTime> for UnixInstant {
    type Error = TryFromError;

    fn try_from(other: std::time::SystemTime) -> Result<Self, Self::Error> {
        let other = other
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .map_err(|_| TryFromError {
                kind: TryFromErrorKind::BeforeEpoch,
            })?
            .as_secs();
        if other > u32::MAX as u64 {
            Err(TryFromError {
                kind: TryFromErrorKind::Overflow,
            })
        } else {
            Ok(Self { secs: other as u32 })
        }
    }
}

impl TryFrom<crate::precise::UnixInstant> for UnixInstant {
    type Error = TryFromError;

    fn try_from(other: crate::precise::UnixInstant) -> Result<Self, Self::Error> {
        let other = other.ns / crate::precise::Duration::SECOND.as_nanos();
        if other > u32::MAX as u64 {
            Err(TryFromError {
                kind: TryFromErrorKind::Overflow,
            })
        } else {
            Ok(Self { secs: other as u32 })
        }
    }
}
