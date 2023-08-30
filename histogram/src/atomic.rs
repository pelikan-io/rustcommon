//! A basic histogram using atomic counters.

use crate::{Bucket, BuildError, Config, Error, _Histograms};
use core::sync::atomic::{AtomicU64, Ordering};

/// A simple concurrent histogram that can be used to track the distribution of
/// occurrences of quantized u64 values.
///
/// Internally it uses 64bit atomic counters to store the counts for each
/// bucket.
pub struct Histogram {
    pub(crate) buckets: Box<[AtomicU64]>,
    pub(crate) config: Config,
}

impl _Histograms for Histogram {
    fn config(&self) -> Config {
        self.config
    }

    fn get_count(&self, index: usize) -> u64 {
        self.buckets[index].load(Ordering::Relaxed)
    }

    fn total_count(&self) -> u128 {
        self.buckets
            .iter()
            .map(|v| v.load(Ordering::Relaxed) as u128)
            .sum()
    }
}

impl Histogram {
    /// Construct a new `atomic::Histogram` from the provided parameters.
    /// * `a` sets bin width in the linear portion, the bin width is `2^a`
    /// * `b` sets the number of divisions in the logarithmic portion to `2^b`.
    /// * `n` sets the max value as `2^n`. Note: when `n` is 64, the max value
    ///   is `u64::MAX`
    ///
    /// # Constraints
    /// * `n` must be less than or equal to 64
    /// * `n` must be greater than `a + b`
    pub fn new(a: u8, b: u8, n: u8) -> Result<Self, BuildError> {
        let config = Config::new(a, b, n)?;

        Ok(Self::from_config(config))
    }

    /// Increment the counter for the bucket corresponding to the provided value
    /// by one.
    pub fn increment(&self, value: u64) -> Result<(), Error> {
        self.add(value, 1)
    }

    /// Add some count to the counter for the bucket corresponding to the
    /// provided value.
    pub fn add(&self, value: u64, count: u64) -> Result<(), Error> {
        let index = self.config.value_to_index(value)?;
        self.buckets[index].fetch_add(count, Ordering::Relaxed);
        Ok(())
    }

    /// Creates a new `Histogram` from the `Config`.
    pub(crate) fn from_config(config: Config) -> Self {
        let mut buckets = Vec::with_capacity(config.total_bins());
        buckets.resize_with(config.total_bins(), || AtomicU64::new(0));

        Self {
            buckets: buckets.into(),
            config,
        }
    }

    /// Get a reference to the raw counters.
    pub fn as_slice(&self) -> &[AtomicU64] {
        &self.buckets
    }

    /// Return a collection of percentiles from this histogram.
    ///
    /// Each percentile should be in the inclusive range `0.0..=100.0`. For
    /// example, the 50th percentile (median) can be found using `50.0`.
    ///
    /// The results will be sorted by the percentile.
    ///
    /// *Note*: concurrent increments may result in percentiles which are not
    /// exactly correct.
    pub fn percentiles(&self, percentiles: &[f64]) -> Result<Vec<(f64, Bucket)>, Error> {
        <Self as _Histograms>::percentiles(self, percentiles)
    }

    /// Return a single percentile from this histogram.
    ///
    /// The percentile should be in the inclusive range `0.0..=100.0`. For
    /// example, the 50th percentile (median) can be found using `50.0`.
    ///
    /// *Note*: concurrent increments may result in percentiles which are not
    /// exactly correct.
    pub fn percentile(&self, percentile: f64) -> Result<Bucket, Error> {
        <Self as _Histograms>::percentile(self, percentile)
    }

    /// Zeros out all the buckets in the histogram.
    ///
    /// *Note*: concurrent increments may result in the histogram not being
    /// clear when this operation completes.
    pub fn clear(&self) {
        for bucket in self.buckets.iter() {
            bucket.store(0, Ordering::Relaxed);
        }
    }
}

impl<'a> IntoIterator for &'a Histogram {
    type Item = Bucket;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Iter {
            index: 0,
            histogram: self,
        }
    }
}

/// An iterator across the histogram buckets.
pub struct Iter<'a> {
    index: usize,
    histogram: &'a Histogram,
}

impl<'a> Iterator for Iter<'a> {
    type Item = Bucket;

    fn next(&mut self) -> Option<<Self as std::iter::Iterator>::Item> {
        let bucket = self.histogram.get_bucket(self.index);
        if bucket.is_some() {
            self.index += 1;
        }

        bucket
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size() {
        assert_eq!(std::mem::size_of::<Histogram>(), 64);
    }

    #[test]
    // Tests percentiles
    fn percentiles() {
        let histogram = Histogram::new(0, 7, 64).unwrap();
        for i in 0..=100 {
            let _ = histogram.increment(i);
            assert_eq!(
                histogram.percentile(0.0),
                Ok(Bucket {
                    count: 1,
                    lower: 0,
                    upper: 0
                })
            );
            assert_eq!(
                histogram.percentile(100.0),
                Ok(Bucket {
                    count: 1,
                    lower: i,
                    upper: i
                })
            );
        }
        assert_eq!(histogram.percentile(25.0).map(|b| b.upper), Ok(25));
        assert_eq!(histogram.percentile(50.0).map(|b| b.upper), Ok(50));
        assert_eq!(histogram.percentile(75.0).map(|b| b.upper), Ok(75));
        assert_eq!(histogram.percentile(90.0).map(|b| b.upper), Ok(90));
        assert_eq!(histogram.percentile(99.0).map(|b| b.upper), Ok(99));
        assert_eq!(histogram.percentile(99.9).map(|b| b.upper), Ok(100));

        assert_eq!(histogram.percentile(-1.0), Err(Error::InvalidPercentile));
        assert_eq!(histogram.percentile(101.0), Err(Error::InvalidPercentile));

        let percentiles: Vec<(f64, u64)> = histogram
            .percentiles(&[50.0, 90.0, 99.0, 99.9])
            .unwrap()
            .iter()
            .map(|(p, b)| (*p, b.upper))
            .collect();

        assert_eq!(
            percentiles,
            vec![(50.0, 50), (90.0, 90), (99.0, 99), (99.9, 100)]
        );

        let _ = histogram.increment(1024);
        assert_eq!(
            histogram.percentile(99.9),
            Ok(Bucket {
                count: 1,
                lower: 1024,
                upper: 1031
            })
        );
    }
}
