use crate::{MICROS_PER_SEC, MILLIS_PER_SEC, NANOS_PER_SEC};
use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Rem, Sub, SubAssign};

/// A duration measured in seconds.
///
/// A duration represents a span of time. Unlike `std::time::Instant` the
/// internal representation uses only seconds in a u32 field to represent
/// the span of time. This means that the max duration is ~136 years.
#[repr(transparent)]
#[derive(Copy, Clone, Default, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration {
    pub(crate) secs: u32,
}

impl Duration {
    /// The maximum representable `coarse::Duration`.
    pub const MAX: Duration = Duration { secs: u32::MAX };

    /// One second as a `coarse::Duration`.
    pub const SECOND: Duration = Duration::from_secs(1);

    /// Creates a new `Duration` from the specified number of whole seconds.
    pub const fn from_secs(secs: u32) -> Self {
        Self { secs }
    }

    /// Returns the number of whole seconds contained by this `Duration`.
    pub const fn as_secs(&self) -> u32 {
        self.secs
    }

    /// Returns the number of seconds contained by this `Duration` as `f64`.
    pub const fn as_secs_f64(&self) -> f64 {
        self.secs as f64
    }

    /// Returns the number of microseconds contained by this `Duration`.
    pub const fn as_micros(&self) -> u64 {
        self.secs as u64 * MICROS_PER_SEC
    }

    /// Returns the number of milliseconds contained by this `Duration`.
    pub const fn as_millis(&self) -> u64 {
        self.secs as u64 * MILLIS_PER_SEC
    }

    /// Returns the number of nanoseconds contained by this `Duration`.
    pub const fn as_nanos(&self) -> u64 {
        self.secs as u64 * NANOS_PER_SEC
    }
}

impl Add<Duration> for Duration {
    type Output = Duration;

    fn add(self, rhs: Duration) -> Self::Output {
        Duration {
            secs: self.secs + rhs.secs,
        }
    }
}

impl AddAssign<Duration> for Duration {
    fn add_assign(&mut self, rhs: Duration) {
        self.secs += rhs.secs;
    }
}

impl Sub<Duration> for Duration {
    type Output = Duration;

    fn sub(self, rhs: Duration) -> Self::Output {
        Duration {
            secs: self.secs - rhs.secs,
        }
    }
}

impl SubAssign<Duration> for Duration {
    fn sub_assign(&mut self, rhs: Duration) {
        self.secs -= rhs.secs;
    }
}

impl Mul<u32> for Duration {
    type Output = Duration;
    fn mul(self, rhs: u32) -> Self::Output {
        Duration {
            secs: self.secs * rhs,
        }
    }
}

impl MulAssign<u32> for Duration {
    fn mul_assign(&mut self, rhs: u32) {
        self.secs *= rhs
    }
}

impl Div<u32> for Duration {
    type Output = Duration;
    fn div(self, rhs: u32) -> Self::Output {
        Duration {
            secs: self.secs / rhs,
        }
    }
}

impl DivAssign<u32> for Duration {
    fn div_assign(&mut self, rhs: u32) {
        self.secs /= rhs
    }
}

impl Rem<Duration> for Duration {
    type Output = Duration;
    fn rem(self, rhs: Duration) -> Self::Output {
        Duration {
            secs: self.secs % rhs.secs,
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

impl TryFrom<core::time::Duration> for Duration {
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
