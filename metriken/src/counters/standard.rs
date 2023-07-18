use crate::*;

/// A counter holds a unsigned 64bit monotonically non-decreasing value. The
/// counter behavior is to wrap on overflow.
///
/// Common examples are the number of operations (requests, reads, ...) or
/// errors.
pub struct Counter {
    value: AtomicU64,
}

impl Metric for Counter {
    fn as_any(&self) -> &(dyn std::any::Any + 'static) {
        self
    }
}

impl Counter {
    /// Initialize a new counter with an initial count of zero.
    pub const fn new() -> Self {
        Self {
            value: AtomicU64::new(0),
        }
    }

    /// Return the current value for the counter.
    pub fn value(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    /// Add one to the counter and return the previous count.
    pub fn increment(&self) -> u64 {
        self.add(1)
    }

    /// Add some count to the counter and return the previous count.
    pub fn add(&self, count: u64) -> u64 {
        self.value.fetch_add(count, Ordering::Relaxed)
    }
}

impl Default for Counter {
    fn default() -> Self {
        Self::new()
    }
}
