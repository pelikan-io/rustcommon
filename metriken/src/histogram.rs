use crate::{Metric, Value};
use parking_lot::RwLock;

use std::sync::OnceLock;

pub use histogram::{Bucket, Config, Error, Snapshot};

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
    config: Config,
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
        let config = match ::histogram::Config::new(grouping_power, max_value_power) {
            Ok(c) => c,
            Err(_) => panic!("invalid histogram config"),
        };

        Self {
            inner: OnceLock::new(),
            config,
        }
    }

    /// Increments the bucket for a corresponding value.
    pub fn increment(&self, value: u64) -> Result<(), Error> {
        self.get_or_init().increment(value)
    }

    pub fn config(&self) -> Config {
        self.config
    }

    /// Create a new snapshot from the histogram.
    pub fn snapshot(&self) -> Option<Snapshot> {
        self.inner.get().map(|h| h.snapshot())
    }

    fn get_or_init(&self) -> &::histogram::AtomicHistogram {
        self.inner
            .get_or_init(|| ::histogram::AtomicHistogram::with_config(&self.config))
    }
}

impl Metric for AtomicHistogram {
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn value(&self) -> Option<Value> {
        Some(Value::Other(self))
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
    config: Config,
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
        let config = match ::histogram::Config::new(grouping_power, max_value_power) {
            Ok(c) => c,
            Err(_e) => panic!("invalid histogram config"),
        };

        Self {
            inner: OnceLock::new(),
            config,
        }
    }

    /// Updates the histogram counts from raw data.
    pub fn update_from(&self, data: &[u64]) -> Result<(), Error> {
        if data.len() != self.config.total_buckets() {
            return Err(Error::IncompatibleParameters);
        }

        let mut histogram = self.get_or_init().write();

        let buckets = histogram.as_mut_slice();
        buckets.copy_from_slice(data);

        Ok(())
    }

    pub fn config(&self) -> Config {
        self.config
    }

    /// Create a new snapshot from the histogram.
    pub fn snapshot(&self) -> Option<Snapshot> {
        self.inner.get().map(|h| h.read().snapshot())
    }

    fn get_or_init(&self) -> &RwLock<::histogram::Histogram> {
        self.inner
            .get_or_init(|| ::histogram::Histogram::with_config(&self.config).into())
    }
}

impl Metric for RwLockHistogram {
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn value(&self) -> Option<Value> {
        Some(Value::Other(self))
    }
}
