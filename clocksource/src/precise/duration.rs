use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Rem, Sub, SubAssign};

/// A duration measured in nanoseconds.
///
/// An duration represents a span of time. Unlike `std::time::Instant` the
/// internal representation uses only nanoseconds in a u64 field to represent
/// the span of time. This means that the max duration is ~584 years.
#[repr(transparent)]
#[derive(Copy, Clone, Default, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration {
    pub(crate) ns: u64,
}

impl Duration {
    /// The maximum representable `precise::Duration`.
    pub const MAX: Duration = Duration { ns: u64::MAX };

    /// One second as a `precise::Duration`.
    pub const SECOND: Duration = Duration::from_nanos(1_000_000_000);

    /// One millisecond as a `precise::Duration`.
    pub const MILLISECOND: Duration = Duration::from_nanos(1_000_000);

    /// One microsecond as a `precise::Duration`.
    pub const MICROSECOND: Duration = Duration::from_nanos(1_000);

    /// One nanosecond as a `precise::Duration`.
    pub const NANOSECOND: Duration = Duration::from_nanos(1);

    /// Create a new `Duration` from a whole number of seconds.
    pub const fn from_secs(secs: u32) -> Self {
        Self {
            ns: secs as u64 * Self::SECOND.as_nanos(),
        }
    }

    /// Create a new `Duration` from a whole number of milliseconds.
    pub const fn from_millis(millis: u32) -> Self {
        Self {
            ns: millis as u64 * Self::MILLISECOND.as_nanos(),
        }
    }

    /// Create a new `Duration` from a whole number of milliseconds.
    ///
    /// *Note*: this will return an error on overflow.
    pub const fn try_from_millis(millis: u64) -> Result<Self, TryFromError> {
        if let Some(ns) = millis.checked_mul(Self::MILLISECOND.as_nanos()) {
            Ok(Self { ns })
        } else {
            Err(TryFromError {
                kind: TryFromErrorKind::Overflow,
            })
        }
    }

    /// Create a new `Duration` from a whole number of microseconds.
    pub const fn from_micros(micros: u32) -> Self {
        Self {
            ns: micros as u64 * Self::MICROSECOND.as_nanos(),
        }
    }

    /// Create a new `Duration` from a whole number of microseconds.
    ///
    /// *Note*: this will return an error on overflow.
    pub const fn try_from_micros(micros: u64) -> Result<Self, TryFromError> {
        if let Some(ns) = micros.checked_mul(Self::MICROSECOND.as_nanos()) {
            Ok(Self { ns })
        } else {
            Err(TryFromError {
                kind: TryFromErrorKind::Overflow,
            })
        }
    }

    /// Create a new `Duration` from a whole number of nanoseconds.
    pub const fn from_nanos(nanos: u64) -> Self {
        Self { ns: nanos }
    }

    /// Returns the whole number of nanoseconds within the `Duration`.
    pub const fn as_nanos(&self) -> u64 {
        self.ns
    }

    /// Returns the whole number of nanoseconds within the `Duration`.
    pub const fn as_micros(&self) -> u64 {
        self.ns / Self::MICROSECOND.as_nanos()
    }

    pub const fn as_millis(&self) -> u64 {
        self.ns / Self::MILLISECOND.as_nanos()
    }

    /// Returns the number of whole seconds represented by this `Duration`.
    ///
    /// This does not include any fractional parts of a second. Use
    /// `subsec_nanos` to get the remainder in nanoseconds. Use `as_secs_f64` if
    /// you want the total number of seconds including the fractional part.
    pub const fn as_secs(&self) -> u64 {
        self.ns / Self::SECOND.as_nanos()
    }

    /// Returns the remaining number of nanoseconds in this `Duration` when
    /// ignoring the whole number of seconds.
    pub const fn subsec_nanos(&self) -> u32 {
        (self.ns % Self::SECOND.as_nanos()) as u32
    }

    /// Returns the total number of seconds represented by this `Duration`.
    pub fn as_secs_f64(&self) -> f64 {
        self.as_secs() as f64 + self.subsec_nanos() as f64 / 1e9
    }

    /// Multiply this `Duration` by a `f64`.
    pub fn mul_f64(self, rhs: f64) -> Self {
        Self {
            ns: (self.ns as f64 * rhs) as u64,
        }
    }
}

impl Add<Duration> for Duration {
    type Output = Duration;

    fn add(self, rhs: Duration) -> Self::Output {
        Duration {
            ns: self.ns + rhs.ns,
        }
    }
}

impl AddAssign<Duration> for Duration {
    fn add_assign(&mut self, rhs: Duration) {
        self.ns += rhs.ns;
    }
}

impl Sub<Duration> for Duration {
    type Output = Duration;

    fn sub(self, rhs: Duration) -> Self::Output {
        Duration {
            ns: self.ns - rhs.ns,
        }
    }
}

impl SubAssign<Duration> for Duration {
    fn sub_assign(&mut self, rhs: Duration) {
        self.ns -= rhs.ns;
    }
}

impl Mul<u64> for Duration {
    type Output = Duration;
    fn mul(self, rhs: u64) -> Self::Output {
        Duration { ns: self.ns * rhs }
    }
}

impl MulAssign<u64> for Duration {
    fn mul_assign(&mut self, rhs: u64) {
        self.ns *= rhs
    }
}

impl Div<u64> for Duration {
    type Output = Duration;
    fn div(self, rhs: u64) -> Self::Output {
        Duration { ns: self.ns / rhs }
    }
}

impl DivAssign<u64> for Duration {
    fn div_assign(&mut self, rhs: u64) {
        self.ns /= rhs
    }
}

impl Rem<Duration> for Duration {
    type Output = Duration;
    fn rem(self, rhs: Duration) -> Self::Output {
        Duration {
            ns: self.ns % rhs.ns,
        }
    }
}

impl From<crate::coarse::Duration> for Duration {
    fn from(other: crate::coarse::Duration) -> Self {
        Self::from_secs(other.as_secs())
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
        if other.as_nanos() > u64::MAX as u128 {
            Err(TryFromError {
                kind: TryFromErrorKind::Overflow,
            })
        } else {
            Ok(Self::from_nanos(other.as_nanos() as u64))
        }
    }
}
