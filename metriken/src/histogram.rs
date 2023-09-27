use crate::{Metric, Value};
use parking_lot::RwLock;

use std::sync::OnceLock;

pub use histogram::{Bucket, Error as HistogramError, Snapshot};

/// A histogram that uses free-running atomic counters to track the distribution
/// of values. They are only useful for recording values and producing
/// [`crate::Snapshot`]s of the histogram state which can then be used for
/// reporting.
///
/// The `AtomicHistogram` should be preferred when individual events are being
/// recorded. The `RwLockHistogram` should be preferred when bulk-updating the
/// histogram from pre-aggregated data with a compatible layout.
pub struct AtomicHistogram {
    inner: OnceLock<histogram::AtomicHistogram>,
    grouping_power: u8,
    max_value_power: u8,
}

impl AtomicHistogram {
    /// Create a new [`::histogram::AtomicHistogram`] with the given parameters.
    ///
    /// # Panics
    /// This will panic if the `grouping_power` and `max_value_power` do not
    /// adhere to the following constraints:
    ///
    /// - `max_value_power` must be in the range 1..=64
    /// - `grouping_power` must be in the range `0..=(max_value_power - 1)`
    pub const fn new(grouping_power: u8, max_value_power: u8) -> Self {
        assert!(::histogram::Config::new(grouping_power, max_value_power).is_ok());

        Self {
            inner: OnceLock::new(),
            grouping_power,
            max_value_power,
        }
    }

    /// Increments the bucket for a corresponding value.
    pub fn increment(&self, value: u64) -> Result<(), HistogramError> {
        self.get_or_init().increment(value)
    }

    /// Create a new snapshot from the histogram.
    pub fn snapshot(&self) -> Option<Snapshot> {
        self.inner.get().map(|h| h.snapshot())
    }

    fn get_or_init(&self) -> &::histogram::AtomicHistogram {
        self.inner.get_or_init(|| {
            ::histogram::AtomicHistogram::new(self.grouping_power, self.max_value_power).unwrap()
        })
    }
}

impl Metric for AtomicHistogram {
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn value(&self) -> Option<Value> {
        Some(Value::AtomicHistogram(self))
    }
}

/// A histogram that uses free-running non-atomic counters to track the
/// distribution of values. They are only useful for bulk recording of values
/// and producing [`crate::Snapshot`]s of the histogram state which can then be
/// used for reporting.
///
/// The `AtomicHistogram` should be preferred when individual events are being
/// recorded. The `RwLockHistogram` should be preferred when bulk-updating the
/// histogram from pre-aggregated data with a compatible layout.
pub struct RwLockHistogram {
    inner: OnceLock<RwLock<histogram::Histogram>>,
    grouping_power: u8,
    max_value_power: u8,
    total_buckets: usize,
}

impl RwLockHistogram {
    /// Create a new [`::histogram::AtomicHistogram`] with the given parameters.
    ///
    /// # Panics
    /// This will panic if the `grouping_power` and `max_value_power` do not
    /// adhere to the following constraints:
    ///
    /// - `max_value_power` must be in the range 1..=64
    /// - `grouping_power` must be in the range `0..=(max_value_power - 1)`
    pub const fn new(grouping_power: u8, max_value_power: u8) -> Self {
        let config = ::histogram::Config::new(grouping_power, max_value_power);

        let total_buckets = match config {
            Ok(c) => c.total_buckets(),
            Err(_e) => panic!("invalid histogram config"),
        };

        Self {
            inner: OnceLock::new(),
            grouping_power,
            max_value_power,
            total_buckets,
        }
    }

    /// Updates the histogram counts from raw data.
    pub fn update_from(&self, data: &[u64]) -> Result<(), HistogramError> {
        if data.len() != self.total_buckets {
            return Err(HistogramError::IncompatibleParameters);
        }

        let mut histogram = self.get_or_init().write();

        let buckets = histogram.as_mut_slice();
        buckets.copy_from_slice(data);

        Ok(())
    }

    /// Create a new snapshot from the histogram.
    pub fn snapshot(&self) -> Option<Snapshot> {
        self.inner.get().map(|h| h.read().snapshot())
    }

    fn get_or_init(&self) -> &RwLock<::histogram::Histogram> {
        self.inner.get_or_init(|| {
            ::histogram::Histogram::new(self.grouping_power, self.max_value_power)
                .unwrap()
                .into()
        })
    }
}

impl Metric for RwLockHistogram {
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn value(&self) -> Option<Value> {
        Some(Value::RwLockHistogram(self))
    }
}
