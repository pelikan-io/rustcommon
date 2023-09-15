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

/// The parameters that determine the histogram bucketing.
/// * `a` sets bin width in the linear portion, the bin width is `2^a`
/// * `b` sets the number of divisions in the logarithmic portion to `2^b`
/// * `n` sets the max value as `2^n`. Note: when `n` is 64, the max value
///   is `u64::MAX`
/// * `a` and `b` together set the cutoff between the linear and logarithmic
///   regions of the histogram. The cutoff is `2^(a + b + 1)`. Values below the
///   cutoff are stored in the linear region. Values between the cutoff and the
///   max value (inclusive) are stored in the logarithmic region.
/// * the absolute error in the linear region is `2^a - 1`
/// * the relative error in the logarithmic region is `2^(-1 * b)`
///
/// # Constraints:
/// * `n` must be in the range `0..=64`
/// * `n` must be greater than `a + b`
///
/// # Example:
/// For a histogram with the parameters `a = 0, b = 7, n = 64`:
/// * The linear region has bins with width of `2^a = 2^0 = 1`. This means it
///   will store values exactly and the relative error in this region is
///   `2^a - 1 = 2^0 - 1 = 1 - 1 = 0`. If we were instead to set `a = 1` the
///   bins will have a width of two, meaning 0 and 1 share a bucket. The
///   absolute error is the difference between the upper and lower bound of any
///   bin in the linear region, in the case of `a = 1` the absolute error would
///   be 1 unit. Since we have set `a = 0`, each bin in the linear region maps
///   to exactly one value, so the absolute error is zero.
/// * The logarithmic region has `2^7 = 128` subdivisions.
/// * The cutoff point, which is the transition between the linear and
///   logarithmic regions, occurs when each subdivision in the logarithmic
///   region is larger than a bin in the linear region.
/// * Since subdivisions set by the `b` parameter span powers of two, the first
///   time a subdivision is wider than a linear bucket is above `2^(a + b + 1)`.
/// * In our example, `a = 0`, so a linear bin is one unit wide. With `b = 7`
///   we have 128 subdivisions. At a minimum, our linear region must cover the
///   range `0..128`. But since 128..256 (the next power of two range) also has
///   128 distinct values, it should also be represented in the linear range as
///   each bin in this region would also have a width of 1 unit.
/// * This holds for all combinations of `a` and `b` and is why the cutoff value
///   `cutoff value = 2^(a + b + 1)`.
/// * If we were to set `a = 1`, we would have linear bins with a width of two
///   units. This would move the cutoff value up to 512 because we can divide
///   from `256..512` into 128 subdivisions (set by `b = 7`) that have a width
///   of two units.
/// * If we were to increase `b` we would also move this cutoff value up as we
///   would allocate additional subdivisions for each logarithmic segment. This
///   moves the point where each subdivision would be wider than a linear bin up
///   because there are finer subdivisions.
#[derive(PartialEq, Debug, Copy, Clone)]
pub struct Parameters {
    pub a: u8,
    pub b: u8,
    pub n: u8,
}

impl Histogram {
    /// Construct a new `Histogram` from the provided parameters. See the
    /// documentation for [`crate::Parameters`] to understand their meaning.
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

        let result: Vec<(f64, Bucket)> = percentiles
            .iter()
            .filter_map(|percentile| {
                let count = (percentile / 100.0 * self.total_count as f64).ceil() as u128;

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

    /// Returns the parameters used to construct this histogram.
    pub fn params(&self) -> Parameters {
        self.config.params()
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
        let mut histogram = Histogram::new(0, 7, 64).unwrap();
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
}
