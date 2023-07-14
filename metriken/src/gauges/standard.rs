use crate::*;

/// A gauge holds a signed 64-bit value and is used to represent metrics which
/// may increase or decrease in value. The behavior is to wrap around on
/// overflow and underflow.
///
/// Common examples are queue depths, temperatures, and usage metrics.
pub struct Gauge {
    value: AtomicI64,
}

impl Metric for Gauge {
    fn as_any(&self) -> &(dyn std::any::Any + 'static) {
        self
    }
}

impl Gauge {
    /// Initialize a new gauge with an initial value of zero.
    pub const fn new() -> Self {
        Self {
            value: AtomicI64::new(0),
        }
    }

    /// Return the current value of the gauge.
    pub fn value(&self) -> i64 {
        self.value.load(Ordering::Relaxed)
    }

    /// Adds one to the current gauge value and returns the previous value.
    pub fn increment(&self) -> i64 {
        self.add(1)
    }

    /// Adds some amount to the current gauge value and returns the previous
    /// value.
    pub fn add(&self, amount: i64) -> i64 {
        self.value.fetch_add(amount, Ordering::Relaxed)
    }

    /// Subtracts one from the current gauge value and returns the previous
    /// value.
    pub fn decrement(&self) -> i64 {
        self.sub(1)
    }

    /// Subtract some amount from the current gauge value and returns the
    /// previous value.
    pub fn sub(&self, amount: i64) -> i64 {
        self.value.fetch_sub(amount, Ordering::Relaxed)
    }
}

impl Default for Gauge {
    fn default() -> Self {
        Self::new()
    }
}
