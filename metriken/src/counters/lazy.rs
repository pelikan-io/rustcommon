use crate::*;

/// A counter holds a unsigned 64bit monotonically increasing counter. The
/// counter behavior is to wrap on overflow.
///
/// Common examples are the number of operations (requests, reads, ...) or
/// errors.
///
/// Unlike a standard `Counter`, a `LazyCounter` will not report a value unless
/// it has been initialized by writing to at least once. This is useful for when
/// you want to declare metrics statically, but only report metrics that are
/// being used.
pub struct LazyCounter {
    inner: OnceLock<AtomicU64>,
}

impl Metric for LazyCounter {
    fn as_any(&self) -> &(dyn std::any::Any + 'static) {
        self
    }
}

impl Metric for &'static LazyCounter {
    fn as_any(&self) -> &(dyn std::any::Any + 'static) {
        self
    }
}

impl LazyCounter {
    /// Initialize a new counter with an initial count of zero.
    pub const fn new() -> Self {
        Self {
            inner: OnceLock::new(),
        }
    }

    /// Return the current value for the counter if it has been written to.
    pub fn value(&self) -> Option<u64> {
        self.inner.get().map(|v| v.load(Ordering::Relaxed))
    }

    /// Add one to the counter and return the previous count.
    pub fn increment(&self) -> u64 {
        self.add(1)
    }

    /// Add some count to the counter and return the previous count.
    pub fn add(&self, count: u64) -> u64 {
        self.get_or_init().fetch_add(count, Ordering::Relaxed)
    }

    fn get_or_init(&self) -> &AtomicU64 {
        self.inner.get_or_init(|| AtomicU64::new(0))
    }
}

impl Default for LazyCounter {
    fn default() -> Self {
        Self::new()
    }
}
