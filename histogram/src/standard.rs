//! A basic histogram using plain counters (no atomics).

use crate::{Bucket, BuildError, Config, Error};

/// A simple histogram that can be used to track the distribution of occurrences
/// of quantized u64 values.
///
/// Internally it uses 64bit counters to store the counts for each bucket.
pub struct Histogram {
    pub(crate) config: Config,
    pub(crate) total_count: u128,
    pub(crate) buckets: Box<[u64]>,
}

impl Histogram {
    /// Construct a new `Histogram` from the provided parameters.
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
    pub fn increment(&mut self, value: u64) -> Result<(), Error> {
        self.add(value, 1)
    }

    /// Add some count to the counter for the bucket corresponding to the
    /// provided value
    pub fn add(&mut self, value: u64, count: u64) -> Result<(), Error> {
        let index = self.config.value_to_index(value)?;
        self.buckets[index] = self.buckets[index].wrapping_add(count);
        self.total_count = self.total_count.wrapping_add(count.into());
        Ok(())
    }

    /// Creates a new `Histogram` from the `Config`.
    pub(crate) fn from_config(config: Config) -> Self {
        let buckets: Box<[u64]> = vec![0; config.total_bins()].into();

        Self {
            config,
            total_count: 0,
            buckets,
        }
    }

    /// Get a reference to the raw counters.
    pub fn as_slice(&self) -> &[u64] {
        &self.buckets
    }

    /// Get a mutable reference to the raw counters.
    pub fn as_mut_slice(&mut self) -> &mut [u64] {
        &mut self.buckets
    }

    /// Return a collection of percentiles from this histogram.
    ///
    /// Each percentile should be in the inclusive range `0.0..=100.0`. For
    /// example, the 50th percentile (median) can be found using `50.0`.
    ///
    /// The results will be sorted by the percentile.
    pub(crate) fn percentiles(&self, percentiles: &[f64]) -> Result<Vec<(f64, Bucket)>, Error> {
        // if the histogram is empty, then we should return an error
        if self.total_count == 0_u128 {
            return Err(Error::Empty);
        }

        // sort the requested percentiles so we can find them in a single pass
        let mut percentiles = percentiles.to_vec();
        percentiles.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // validate all the percentiles
        for percentile in &percentiles {
            if !(0.0..=100.0).contains(percentile) {
                return Err(Error::InvalidPercentile);
            }
        }

        let mut bucket_idx = 0;
        let mut partial_sum = self.buckets[bucket_idx] as u128;

        let result = percentiles
            .iter()
            .filter_map(|percentile| {
                let count = (percentile / 100.0 * self.total_count as f64).ceil() as u128;

                while bucket_idx < (self.buckets.len() - 1) {
                    // found the matching bucket index for this percentile
                    if partial_sum >= count {
                        return Some((
                            *percentile,
                            Bucket {
                                count: self.buckets[bucket_idx],
                                lower: self.config.index_to_lower_bound(bucket_idx),
                                upper: self.config.index_to_upper_bound(bucket_idx),
                            },
                        ));
                    }

                    // otherwise, increment the bucket index, partial sum, and loop
                    bucket_idx += 1;
                    partial_sum += self.buckets[bucket_idx] as u128;
                }

                None
            })
            .collect();

        Ok(result)
    }

    /// Return a single percentile from this histogram.
    ///
    /// The percentile should be in the inclusive range `0.0..=100.0`. For
    /// example, the 50th percentile (median) can be found using `50.0`.
    pub fn percentile(&self, percentile: f64) -> Result<Bucket, Error> {
        self.percentiles(&[percentile])
            .map(|v| v.first().unwrap().1)
    }

    /// Merge the counts from the other histogram into this histogram.
    pub fn merge(&mut self, other: &Histogram) -> Result<(), Error> {
        if self.config.params() != other.config.params() {
            return Err(Error::MergeIncompatibleParameters);
        }

        for (this, other) in self.buckets.iter_mut().zip(other.buckets.iter()) {
            *this = this.wrapping_add(*other);
        }

        Ok(())
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
        if self.index >= self.histogram.buckets.len() {
            return None;
        }

        let bucket = Bucket {
            count: self.histogram.buckets[self.index],
            lower: self.histogram.config.index_to_lower_bound(self.index),
            upper: self.histogram.config.index_to_upper_bound(self.index),
        };

        self.index += 1;

        Some(bucket)
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
        let mut histogram = Histogram::new(0, 7, 64).unwrap();
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
