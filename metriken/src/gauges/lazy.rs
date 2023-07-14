use crate::*;

/// A gauge holds a signed 64-bit value and is used to represent metrics which
/// may increase or decrease in value. The behavior is to wrap around on
/// overflow and underflow.
///
/// Common examples are queue depths, temperatures, and usage metrics.
///
/// Unlike a standard `Gauge`, a `LazyGauge` will not report a value unless it
/// has been initialized by writing to at least once. This is useful for when
/// you want to declare metrics statically, but only report metrics that are
/// being used.
pub struct LazyGauge {
    inner: OnceLock<AtomicI64>,
}

impl Metric for LazyGauge {
    fn as_any(&self) -> &(dyn std::any::Any + 'static) {
        self
    }
}

impl LazyGauge {
    /// Initialize a new gauge with an initial value of zero.
    pub const fn new() -> Self {
        Self {
            inner: OnceLock::new(),
        }
    }

    /// Return the current value of the gauge.
    pub fn value(&self) -> Option<i64> {
        self.inner.get().map(|v| v.load(Ordering::Relaxed))
    }

    /// Adds one to the current gauge value and returns the previous value.
    pub fn increment(&self) -> i64 {
        self.add(1)
    }

    /// Adds some amount to the current gauge value and returns the previous
    /// value.
    pub fn add(&self, amount: i64) -> i64 {
        self.get_or_init().fetch_add(amount, Ordering::Relaxed)
    }

    /// Subtracts one from the current gauge value and returns the previous
    /// value.
    pub fn decrement(&self) -> i64 {
        self.sub(1)
    }

    /// Subtract some amount from the current gauge value and returns the
    /// previous value.
    pub fn sub(&self, amount: i64) -> i64 {
        self.get_or_init().fetch_sub(amount, Ordering::Relaxed)
    }

    fn get_or_init(&self) -> &AtomicI64 {
        self.inner.get_or_init(|| AtomicI64::new(0))
    }
}

impl Default for LazyGauge {
    fn default() -> Self {
        Self::new()
    }
}
