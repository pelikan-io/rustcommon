use crate::{Config, Error, Histogram, Snapshot};
use core::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

/// A histogram that uses atomic 64bit counters for each bucket.
///
/// Unlike the non-atomic variant, it cannot be used directly to report
/// percentiles. Instead, a snapshot must be taken which captures the state of
/// the histogram at a point in time.
pub struct AtomicHistogram {
    config: Config,
    start: SystemTime,
    buckets: Box<[AtomicU64]>,
}

impl AtomicHistogram {
    /// Construct a new atomic histogram from the provided parameters. See the
    /// documentation for [`crate::Config`] to understand their meaning.
    pub fn new(p: u8, n: u8) -> Result<Self, Error> {
        let config = Config::new(p, n)?;

        Ok(Self::with_config(&config))
    }

    /// Creates a new atomic histogram using a provided [`crate::Config`].
    pub fn with_config(config: &Config) -> Self {
        let mut buckets = Vec::with_capacity(config.total_buckets());
        buckets.resize_with(config.total_buckets(), || AtomicU64::new(0));

        Self {
            config: *config,
            start: SystemTime::now(),
            buckets: buckets.into(),
        }
    }

    /// Increment the bucket that contains the value by one.
    ///
    /// This is a convenience method that uses `Instant::now()` as the time
    /// associated with the observation. If you already have a timestamp, you
    /// may wish to use `increment_at` instead.
    pub fn increment(&self, value: u64) -> Result<(), Error> {
        self.add(value, 1)
    }

    /// Increment the bucket that contains the value by some count.
    ///
    /// This is a convenience method that uses `Instant::now()` as the time
    /// associated with the observation. If you already have a timestamp, you
    /// may wish to use the `add_at` instead.
    pub fn add(&self, value: u64, count: u64) -> Result<(), Error> {
        let index = self.config.value_to_index(value)?;
        self.buckets[index].fetch_add(count, Ordering::Relaxed);
        Ok(())
    }

    /// Produce a snapshot from this histogram.
    pub fn snapshot(&self) -> Snapshot {
        let end = SystemTime::now();

        let buckets: Vec<u64> = self
            .buckets
            .iter()
            .map(|bucket| bucket.load(Ordering::Relaxed))
            .collect();

        let histogram = Histogram {
            config: self.config,
            start: self.start,
            buckets: buckets.into(),
        };

        Snapshot { end, histogram }
    }
}

#[cfg(test)]
mod test {
    use crate::*;

    #[test]
    fn size() {
        assert_eq!(std::mem::size_of::<AtomicHistogram>(), 64);
    }

    #[test]
    // Tests percentiles
    fn percentiles() {
        let histogram = AtomicHistogram::new(7, 64).unwrap();
        for i in 0..=100 {
            let _ = histogram.increment(i);
            assert_eq!(
                histogram.snapshot().percentile(0.0),
                Ok(Bucket {
                    count: 1,
                    range: 0..=0,
                })
            );
            assert_eq!(
                histogram.snapshot().percentile(100.0),
                Ok(Bucket {
                    count: 1,
                    range: i..=i,
                })
            );
        }
        assert_eq!(
            histogram.snapshot().percentile(25.0).map(|b| b.end()),
            Ok(25)
        );
        assert_eq!(
            histogram.snapshot().percentile(50.0).map(|b| b.end()),
            Ok(50)
        );
        assert_eq!(
            histogram.snapshot().percentile(75.0).map(|b| b.end()),
            Ok(75)
        );
        assert_eq!(
            histogram.snapshot().percentile(90.0).map(|b| b.end()),
            Ok(90)
        );
        assert_eq!(
            histogram.snapshot().percentile(99.0).map(|b| b.end()),
            Ok(99)
        );
        assert_eq!(
            histogram.snapshot().percentile(99.9).map(|b| b.end()),
            Ok(100)
        );

        assert_eq!(
            histogram.snapshot().percentile(-1.0),
            Err(Error::InvalidPercentile)
        );
        assert_eq!(
            histogram.snapshot().percentile(101.0),
            Err(Error::InvalidPercentile)
        );

        let percentiles: Vec<(f64, u64)> = histogram
            .snapshot()
            .percentiles(&[50.0, 90.0, 99.0, 99.9])
            .unwrap()
            .iter()
            .map(|(p, b)| (*p, b.end()))
            .collect();

        assert_eq!(
            percentiles,
            vec![(50.0, 50), (90.0, 90), (99.0, 99), (99.9, 100)]
        );

        let _ = histogram.increment(1024);
        assert_eq!(
            histogram.snapshot().percentile(99.9),
            Ok(Bucket {
                count: 1,
                range: 1024..=1031,
            })
        );
    }
}
