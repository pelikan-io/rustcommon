use core::ops::{Add, AddAssign, Sub, SubAssign};

use super::Duration;

/// A measurement of a monotonically nodecreasing clock in nanoseconds.
///
/// It is opaque and useful only with the duration types.
///
/// Unlike `std::time::Instant` the internal representation use only nanoseconds
/// in a `u64` field to hold the clock reading. This means that they will wrap
/// after ~584 years.
///
/// As with `std::time::Instant`, instants are not guaranteed to be steady. They
/// are taken from a clock which is subject to phase and frequency adjustments.
/// This means that they may jump forward or speed up or slow down. Barring any
/// platform bugs, it is expected that they are always monotonically
/// nondecreasing.
///
/// The size of a `precise::Instant` is always the same as a `u64`.
#[repr(transparent)]
#[derive(Copy, Clone, Default, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instant {
    pub(crate) ns: u64,
}

impl Instant {
    /// Return an `Instant` that represents the current moment.
    pub fn now() -> Self {
        crate::sys::monotonic::precise()
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
        self.ns.checked_sub(earlier.ns).map(|ns| Duration { ns })
    }

    pub fn checked_sub(&self, duration: Duration) -> Option<Self> {
        self.ns.checked_sub(duration.ns).map(|ns| Self { ns })
    }
}

impl Add<Duration> for Instant {
    type Output = Instant;

    fn add(self, rhs: Duration) -> Self::Output {
        Instant {
            ns: self.ns + rhs.ns,
        }
    }
}

impl Add<core::time::Duration> for Instant {
    type Output = Instant;

    fn add(self, rhs: core::time::Duration) -> Self::Output {
        Instant {
            ns: self.ns + rhs.as_nanos() as u64,
        }
    }
}

impl Sub<Instant> for Instant {
    type Output = Duration;

    fn sub(self, rhs: Instant) -> Self::Output {
        Duration {
            ns: self.ns - rhs.ns,
        }
    }
}

impl AddAssign<Duration> for Instant {
    fn add_assign(&mut self, rhs: Duration) {
        self.ns += rhs.ns;
    }
}

impl Sub<Duration> for Instant {
    type Output = Instant;

    fn sub(self, rhs: Duration) -> Self::Output {
        Instant {
            ns: self.ns - rhs.ns,
        }
    }
}

impl SubAssign<Duration> for Instant {
    fn sub_assign(&mut self, rhs: Duration) {
        self.ns -= rhs.ns;
    }
}

impl AddAssign<core::time::Duration> for Instant {
    fn add_assign(&mut self, rhs: core::time::Duration) {
        self.ns += rhs.as_nanos() as u64;
    }
}

impl Sub<core::time::Duration> for Instant {
    type Output = Instant;

    fn sub(self, rhs: core::time::Duration) -> Self::Output {
        Instant {
            ns: self.ns - rhs.as_nanos() as u64,
        }
    }
}

impl SubAssign<core::time::Duration> for Instant {
    fn sub_assign(&mut self, rhs: core::time::Duration) {
        self.ns -= rhs.as_nanos() as u64;
    }
}

impl From<crate::coarse::Instant> for Instant {
    fn from(other: crate::coarse::Instant) -> Self {
        Self {
            ns: other.secs as u64 * super::Duration::NANOSECOND.as_nanos(),
        }
    }
}
