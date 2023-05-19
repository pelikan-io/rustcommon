use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Rem, Sub, SubAssign};

/// An duration represents a span of time. Unlike `std::time::Instant` the
/// internal representation uses only nanoseconds in a u64 field to represent
/// the span of time. This means that the max duration is ~584 years.
#[repr(transparent)]
#[derive(Copy, Clone, Default, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration {
    pub(crate) ns: u64,
}

impl Duration {
    pub fn from_secs(secs: u32) -> Self {
        Self { ns: secs as u64 * 1_000_000_000 }
    }

    pub fn from_millis(millis: u64) -> Self {
        Self { ns: millis * 1_000_000 }
    }

    pub fn from_micros(micros: u64) -> Self {
        Self { ns: micros * 1_000 }
    }

    pub fn from_nanos(nanos: u64) -> Self {
        Self { ns: nanos }
    }

    pub fn as_nanos(&self) -> u64 {
        self.ns
    }

    pub fn as_secs_f64(&self) -> f64 {
        (self.ns / 1_000_000_000) as f64 + (self.ns % 1_000_000_000) as f64 / 1e9
    }

    pub fn mul_f64(self, rhs: f64) -> Self {
        Self { ns: (self.ns as f64 * rhs) as u64 }
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
