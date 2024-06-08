use crate::{Bucket, Config, Error, SparseHistogram};

/// A histogram that uses plain 64bit counters for each bucket.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Histogram {
    pub(crate) config: Config,
    pub(crate) buckets: Box<[u64]>,
}

impl Histogram {
    /// Construct a new histogram from the provided parameters. See the
    /// documentation for [`crate::Config`] to understand their meaning.
    pub fn new(grouping_power: u8, max_value_power: u8) -> Result<Self, Error> {
        let config = Config::new(grouping_power, max_value_power)?;

        Ok(Self::with_config(&config))
    }

    /// Creates a new histogram using a provided [`crate::Config`].
    pub fn with_config(config: &Config) -> Self {
        let buckets: Box<[u64]> = vec![0; config.total_buckets()].into();

        Self {
            config: *config,
            buckets,
        }
    }

    /// Creates a new histogram using a provided [`crate::Config`] and the
    /// provided collection of buckets.
    pub fn from_buckets(
        grouping_power: u8,
        max_value_power: u8,
        buckets: Vec<u64>,
    ) -> Result<Self, Error> {
        let config = Config::new(grouping_power, max_value_power)?;

        if config.total_buckets() != buckets.len() {
            return Err(Error::IncompatibleParameters);
        }

        Ok(Self {
            config,
            buckets: buckets.into(),
        })
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
    pub fn percentiles(&self, percentiles: &[f64]) -> Result<Option<Vec<(f64, Bucket)>>, Error> {
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

        // empty histogram, no percentiles available
        if total_count == 0 {
            return Ok(None);
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

        Ok(Some(result))
    }

    /// Return a single percentile from this histogram.
    ///
    /// The percentile should be in the inclusive range `0.0..=100.0`. For
    /// example, the 50th percentile (median) can be found using `50.0`.
    pub fn percentile(&self, percentile: f64) -> Result<Option<Bucket>, Error> {
        self.percentiles(&[percentile])
            .map(|v| v.map(|x| x.first().unwrap().1.clone()))
    }

    /// Returns a new histogram with a reduced grouping power. The reduced
    /// grouping power should lie in the range (0..existing grouping power).
    ///
    /// The difference in grouping powers determines how much histogram size
    /// is reduced by, with every step approximately halving the total
    /// number of buckets (and hence total size of the histogram), while
    /// doubling the relative error.
    ///
    /// This works by iterating over every bucket in the existing histogram
    /// and inserting the contained values into the new histogram. While we
    /// do not know the exact values of the data points (only that they lie
    /// within the bucket's range), it does not matter since the bucket is
    /// not split during downsampling and any value can be used.
    pub fn downsample(&self, grouping_power: u8) -> Result<Histogram, Error> {
        if grouping_power >= self.config.grouping_power() {
            return Err(Error::MaxPowerTooLow);
        }

        let mut histogram = Histogram::new(grouping_power, self.config.max_value_power())?;
        for (i, n) in self.as_slice().iter().enumerate() {
            // Skip empty buckets
            if *n != 0 {
                let val = self.config.index_to_lower_bound(i);
                histogram.add(val, *n)?;
            }
        }

        Ok(histogram)
    }

    /// Adds the other histogram to this histogram and returns the result as a
    /// new histogram.
    ///
    /// An error is returned if the two histograms have incompatible parameters
    /// or if there is an overflow.
    pub fn checked_add(&self, other: &Histogram) -> Result<Histogram, Error> {
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
    pub fn wrapping_add(&self, other: &Histogram) -> Result<Histogram, Error> {
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
    pub fn checked_sub(&self, other: &Histogram) -> Result<Histogram, Error> {
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
    pub fn wrapping_sub(&self, other: &Histogram) -> Result<Histogram, Error> {
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

impl From<&SparseHistogram> for Histogram {
    fn from(other: &SparseHistogram) -> Self {
        let mut histogram = Histogram::with_config(&other.config);

        for (index, count) in other.index.iter().zip(other.count.iter()) {
            histogram.buckets[*index] = *count;
        }

        histogram
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn size() {
        assert_eq!(std::mem::size_of::<Histogram>(), 48);
    }

    #[test]
    // Tests percentiles
    fn percentiles() {
        let mut histogram = Histogram::new(7, 64).unwrap();

        assert_eq!(histogram.percentile(50.0).unwrap(), None);
        assert_eq!(
            histogram.percentiles(&[50.0, 90.0, 99.0, 99.9]).unwrap(),
            None
        );

        for i in 0..=100 {
            let _ = histogram.increment(i);
            assert_eq!(
                histogram.percentile(0.0),
                Ok(Some(Bucket {
                    count: 1,
                    range: 0..=0,
                }))
            );
            assert_eq!(
                histogram.percentile(100.0),
                Ok(Some(Bucket {
                    count: 1,
                    range: i..=i,
                }))
            );
        }
        assert_eq!(histogram.percentile(25.0).map(|b| b.unwrap().end()), Ok(25));
        assert_eq!(histogram.percentile(50.0).map(|b| b.unwrap().end()), Ok(50));
        assert_eq!(histogram.percentile(75.0).map(|b| b.unwrap().end()), Ok(75));
        assert_eq!(histogram.percentile(90.0).map(|b| b.unwrap().end()), Ok(90));
        assert_eq!(histogram.percentile(99.0).map(|b| b.unwrap().end()), Ok(99));
        assert_eq!(
            histogram.percentile(99.9).map(|b| b.unwrap().end()),
            Ok(100)
        );

        assert_eq!(histogram.percentile(-1.0), Err(Error::InvalidPercentile));
        assert_eq!(histogram.percentile(101.0), Err(Error::InvalidPercentile));

        let percentiles: Vec<(f64, u64)> = histogram
            .percentiles(&[50.0, 90.0, 99.0, 99.9])
            .unwrap()
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
            Ok(Some(Bucket {
                count: 1,
                range: 1024..=1031,
            }))
        );
    }

    #[test]
    #[ignore = "this test is flaky (see issue #100)"]
    // Tests downsampling
    fn downsample() {
        let mut histogram = Histogram::new(8, 32).unwrap();
        let mut vals: Vec<u64> = Vec::with_capacity(10000);
        let mut rng = rand::thread_rng();

        // Generate 10,000 values to store in a sorted array and a histogram
        for _ in 0..vals.capacity() {
            let v: u64 = rng.gen_range(1..2_u64.pow(histogram.config.max_value_power() as u32));
            vals.push(v);
            let _ = histogram.increment(v);
        }
        vals.sort();

        // List of percentiles to query and validate
        let mut percentiles: Vec<f64> = Vec::with_capacity(109);
        for i in 20..99 {
            percentiles.push(i as f64);
        }
        let mut tail = vec![
            99.1, 99.2, 99.3, 99.4, 99.5, 99.6, 99.7, 99.8, 99.9, 99.99, 100.0,
        ];
        percentiles.append(&mut tail);

        // Downsample and check the percentiles lie within error margin
        let h = histogram.clone();
        let grouping_power = histogram.config.grouping_power();
        for factor in 1..grouping_power {
            let error = histogram.config.error();

            for p in &percentiles {
                let v = vals[((*p / 100.0 * (vals.len() as f64)) as usize) - 1];

                // Value and relative error from full histogram
                let vhist = histogram.percentile(*p).unwrap().unwrap().end();
                let e = (v.abs_diff(vhist) as f64) * 100.0 / (v as f64);
                assert!(e < error);
            }

            histogram = h.downsample(grouping_power - factor).unwrap();
        }
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

    #[test]
    // Test creating the histogram from buckets
    fn from_buckets() {
        let mut histogram = Histogram::new(8, 32).unwrap();
        for i in 0..=100 {
            let _ = histogram.increment(i);
        }

        let buckets = histogram.as_slice();
        let constructed = Histogram::from_buckets(8, 32, buckets.to_vec()).unwrap();

        assert!(constructed == histogram);
    }
}
