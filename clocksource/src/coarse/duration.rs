use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Rem, Sub, SubAssign};

/// An duration represents a span of time. Unlike `std::time::Instant` the
/// internal representation uses only nanoseconds in a u64 field to represent
/// the span of time. This means that the max duration is ~584 years.
#[repr(transparent)]
#[derive(Copy, Clone, Default, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration {
    pub(crate) secs: u32,
}

impl Duration {
    pub fn from_secs(secs: u32) -> Self {
        Self { secs }
    }

    pub fn as_secs(&self) -> u32 {
        self.secs
    }

    pub fn as_secs_f64(&self) -> f64 {
        self.secs as f64
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
