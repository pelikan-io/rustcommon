use crate::{Bucket, BuildError, Config, Error, Snapshot};
use std::time::SystemTime;

/// A histogram that uses plain 64bit counters for each bucket.
#[derive(Clone, Debug, PartialEq)]
pub struct Histogram {
    pub(crate) config: Config,
    pub(crate) start: SystemTime,
    pub(crate) buckets: Box<[u64]>,
}

impl Histogram {
    /// Construct a new histogram from the provided parameters. See the
    /// documentation for [`crate::Config`] to understand their meaning.
    pub fn new(grouping_power: u8, max_value_power: u8) -> Result<Self, BuildError> {
        let config = Config::new(grouping_power, max_value_power)?;

        Ok(Self::with_config(&config))
    }

    /// Creates a new histogram using a provided [`crate::Config`].
    pub fn with_config(config: &Config) -> Self {
        let buckets: Box<[u64]> = vec![0; config.total_buckets()].into();

        Self {
            config: *config,
            start: SystemTime::now(),
            buckets,
        }
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
        Ok(())
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
    pub fn percentiles(&self, percentiles: &[f64]) -> Result<Vec<(f64, Bucket)>, Error> {
        // get the total count
        let total_count: u128 = self.buckets.iter().map(|v| *v as u128).sum();

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

        let result: Vec<(f64, Bucket)> = percentiles
            .iter()
            .filter_map(|percentile| {
                let count = (percentile / 100.0 * total_count as f64).ceil() as u128;

                loop {
                    // found the matching bucket index for this percentile
                    if partial_sum >= count {
                        return Some((
                            *percentile,
                            Bucket {
                                count: self.buckets[bucket_idx],
                                range: self.config.index_to_range(bucket_idx),
                            },
                        ));
                    }

                    // check if we have reached the end of the buckets
                    if bucket_idx == (self.buckets.len() - 1) {
                        break;
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
            .map(|v| v.first().unwrap().1.clone())
    }

    /// Produce a snapshot from this histogram.
    pub fn snapshot(&self) -> Snapshot {
        let end = SystemTime::now();

        Snapshot {
            end,
            histogram: self.clone(),
        }
    }

    /// Adds the other histogram to this histogram and returns the result as a
    /// new histogram.
    ///
    /// An error is returned if the two histograms have incompatible parameters
    /// or if there is an overflow.
    pub(crate) fn checked_add(&self, other: &Histogram) -> Result<Histogram, Error> {
        if self.config != other.config {
            return Err(Error::IncompatibleParameters);
        }

        let mut result = self.clone();

        for (this, other) in result.buckets.iter_mut().zip(other.buckets.iter()) {
            *this = this.checked_add(*other).ok_or(Error::Overflow)?;
        }

        Ok(result)
    }

    /// Adds the other histogram to this histogram and returns the result as a
    /// new histogram.
    ///
    /// An error is returned if the two histograms have incompatible parameters.
    pub(crate) fn wrapping_add(&self, other: &Histogram) -> Result<Histogram, Error> {
        if self.config != other.config {
            return Err(Error::IncompatibleParameters);
        }

        let mut result = self.clone();

        for (this, other) in result.buckets.iter_mut().zip(other.buckets.iter()) {
            *this = this.wrapping_add(*other);
        }

        Ok(result)
    }

    /// Subtracts the other histogram from this histogram and returns the result
    /// as a new histogram.
    ///
    /// An error is returned if the two histograms have incompatible parameters
    /// or if there is an overflow.
    pub(crate) fn checked_sub(&self, other: &Histogram) -> Result<Histogram, Error> {
        if self.config != other.config {
            return Err(Error::IncompatibleParameters);
        }

        let mut result = self.clone();

        for (this, other) in result.buckets.iter_mut().zip(other.buckets.iter()) {
            *this = this.checked_sub(*other).ok_or(Error::Overflow)?;
        }

        Ok(result)
    }

    /// Subtracts the other histogram from this histogram and returns the result
    /// as a new histogram.
    ///
    /// An error is returned if the two histograms have incompatible parameters.
    pub(crate) fn wrapping_sub(&self, other: &Histogram) -> Result<Histogram, Error> {
        if self.config != other.config {
            return Err(Error::IncompatibleParameters);
        }

        let mut result = self.clone();

        for (this, other) in result.buckets.iter_mut().zip(other.buckets.iter()) {
            *this = this.wrapping_sub(*other);
        }

        Ok(result)
    }

    /// Returns the bucket configuration of the histogram.
    pub fn config(&self) -> Config {
        self.config
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
            range: self.histogram.config.index_to_range(self.index),
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
        let mut histogram = Histogram::new(7, 64).unwrap();
        for i in 0..=100 {
            let _ = histogram.increment(i);
            assert_eq!(
                histogram.percentile(0.0),
                Ok(Bucket {
                    count: 1,
                    range: 0..=0,
                })
            );
            assert_eq!(
                histogram.percentile(100.0),
                Ok(Bucket {
                    count: 1,
                    range: i..=i,
                })
            );
        }
        assert_eq!(histogram.percentile(25.0).map(|b| b.end()), Ok(25));
        assert_eq!(histogram.percentile(50.0).map(|b| b.end()), Ok(50));
        assert_eq!(histogram.percentile(75.0).map(|b| b.end()), Ok(75));
        assert_eq!(histogram.percentile(90.0).map(|b| b.end()), Ok(90));
        assert_eq!(histogram.percentile(99.0).map(|b| b.end()), Ok(99));
        assert_eq!(histogram.percentile(99.9).map(|b| b.end()), Ok(100));

        assert_eq!(histogram.percentile(-1.0), Err(Error::InvalidPercentile));
        assert_eq!(histogram.percentile(101.0), Err(Error::InvalidPercentile));

        let percentiles: Vec<(f64, u64)> = histogram
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
            histogram.percentile(99.9),
            Ok(Bucket {
                count: 1,
                range: 1024..=1031,
            })
        );
    }

    // Return four histograms (three with identical configs and one with a
    // different config) for testing add and subtract. One of the histograms
    // should be populated with the maximum u64 value to cause overflows.
    fn build_histograms() -> (Histogram, Histogram, Histogram, Histogram) {
        let mut h1 = Histogram::new(1, 3).unwrap();
        let mut h2 = Histogram::new(1, 3).unwrap();
        let mut h3 = Histogram::new(1, 3).unwrap();
        let h4 = Histogram::new(7, 32).unwrap();

        for i in 0..h1.config().total_buckets() {
            h1.as_mut_slice()[i] = 1;
            h2.as_mut_slice()[i] = 1;
            h3.as_mut_slice()[i] = u64::MAX;
        }

        (h1, h2, h3, h4)
    }

    #[test]
    // Tests checked add
    fn checked_add() {
        let (h, h_good, h_overflow, h_mismatch) = build_histograms();

        assert_eq!(
            h.checked_add(&h_mismatch),
            Err(Error::IncompatibleParameters)
        );

        let r = h.checked_add(&h_good).unwrap();
        assert_eq!(r.as_slice(), &[2, 2, 2, 2, 2, 2]);

        assert_eq!(h.checked_add(&h_overflow), Err(Error::Overflow));
    }

    #[test]
    // Tests wrapping add
    fn wrapping_add() {
        let (h, h_good, h_overflow, h_mismatch) = build_histograms();

        assert_eq!(
            h.wrapping_add(&h_mismatch),
            Err(Error::IncompatibleParameters)
        );

        let r = h.wrapping_add(&h_good).unwrap();
        assert_eq!(r.as_slice(), &[2, 2, 2, 2, 2, 2]);

        let r = h.wrapping_add(&h_overflow).unwrap();
        assert_eq!(r.as_slice(), &[0, 0, 0, 0, 0, 0]);
    }

    #[test]
    // Tests checked sub
    fn checked_sub() {
        let (h, h_good, h_overflow, h_mismatch) = build_histograms();

        assert_eq!(
            h.checked_sub(&h_mismatch),
            Err(Error::IncompatibleParameters)
        );

        let r = h.checked_sub(&h_good).unwrap();
        assert_eq!(r.as_slice(), &[0, 0, 0, 0, 0, 0]);

        assert_eq!(h.checked_add(&h_overflow), Err(Error::Overflow));
    }

    #[test]
    // Tests wrapping sub
    fn wrapping_sub() {
        let (h, h_good, h_overflow, h_mismatch) = build_histograms();

        assert_eq!(
            h.wrapping_sub(&h_mismatch),
            Err(Error::IncompatibleParameters)
        );

        let r = h.wrapping_sub(&h_good).unwrap();
        assert_eq!(r.as_slice(), &[0, 0, 0, 0, 0, 0]);

        let r = h.wrapping_sub(&h_overflow).unwrap();
        assert_eq!(r.as_slice(), &[2, 2, 2, 2, 2, 2]);
    }
}
