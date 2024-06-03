use crate::{Bucket, Config, Error, Histogram};

/// This histogram is a sparse, columnar representation of the regular
/// Histogram. It is significantly smaller than a regular Histogram
/// when a large number of buckets are zero, which is a frequent
/// occurence. It stores an individual vector for each field
/// of non-zero buckets. Assuming index[0] = n, (index[0], count[0])
/// corresponds to the nth bucket.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct SparseHistogram {
    /// parameters representing the resolution and the range of
    /// the histogram tracking request latencies
    pub config: Config,
    /// indices for the non-zero buckets in the histogram
    pub index: Vec<usize>,
    /// histogram bucket counts corresponding to the indices
    pub count: Vec<u64>,
}

impl SparseHistogram {
    /// Construct a new histogram from the provided parameters. See the
    /// documentation for [`crate::Config`] to understand their meaning.
    pub fn new(grouping_power: u8, max_value_power: u8) -> Result<Self, Error> {
        let config = Config::new(grouping_power, max_value_power)?;

        Ok(Self::with_config(&config))
    }

    /// Creates a new histogram using a provided [`crate::Config`].
    pub fn with_config(config: &Config) -> Self {
        Self {
            config: *config,
            index: Vec::new(),
            count: Vec::new(),
        }
    }

    /// Helper function to store a bucket in the histogram.
    fn add_bucket(&mut self, idx: usize, n: u64) {
        if n != 0 {
            self.index.push(idx);
            self.count.push(n);
        }
    }

    /// Adds the other histogram to this histogram and returns the result as a
    /// new histogram.
    ///
    /// An error is returned if the two histograms have incompatible parameters.
    /// Buckets which have values in both histograms are allowed to wrap.
    #[allow(clippy::comparison_chain)]
    pub fn wrapping_add(&self, h: &SparseHistogram) -> Result<SparseHistogram, Error> {
        if self.config != h.config {
            return Err(Error::IncompatibleParameters);
        }

        let mut histogram = SparseHistogram::with_config(&self.config);

        // Sort and merge buckets from both histograms
        let (mut i, mut j) = (0, 0);
        while i < self.index.len() && j < h.index.len() {
            let (k1, v1) = (self.index[i], self.count[i]);
            let (k2, v2) = (h.index[j], h.count[j]);

            if k1 == k2 {
                histogram.add_bucket(k1, v1 + v2);
                (i, j) = (i + 1, j + 1);
            } else if k1 < k2 {
                histogram.add_bucket(k1, v1);
                i += 1;
            } else {
                histogram.add_bucket(k2, v2);
                j += 1;
            }
        }

        // Fill remaining values, if any, from the left histogram
        if i < self.index.len() {
            histogram.index.extend(&self.index[i..self.index.len()]);
            histogram.count.extend(&self.count[i..self.count.len()]);
        }

        // Fill remaining values, if any, from the right histogram
        if j < h.index.len() {
            histogram.index.extend(&h.index[j..h.index.len()]);
            histogram.count.extend(&h.count[j..h.count.len()]);
        }

        Ok(histogram)
    }

    /// Subtracts the other histogram to this histogram and returns the result as a
    /// new histogram. The other histogram is expected to be a subset of the current
    /// histogram, i.e., for every bucket in the other histogram should have a
    /// count less than or equal to the corresponding bucket in this histogram.
    ///
    /// An error is returned if the two histograms have incompatible parameters
    /// or if the other histogram is not a subset of this histogram.
    #[allow(clippy::comparison_chain)]
    pub fn checked_sub(&self, h: &SparseHistogram) -> Result<SparseHistogram, Error> {
        if self.config != h.config {
            return Err(Error::IncompatibleParameters);
        }

        let mut histogram = SparseHistogram::with_config(&self.config);

        // Sort and merge buckets from both histograms
        let (mut i, mut j) = (0, 0);
        while i < self.index.len() && j < h.index.len() {
            let (k1, v1) = (self.index[i], self.count[i]);
            let (k2, v2) = (h.index[j], h.count[j]);

            if k1 == k2 {
                let v = v1.checked_sub(v2).ok_or(Error::Underflow)?;
                if v != 0 {
                    histogram.add_bucket(k1, v);
                }
                (i, j) = (i + 1, j + 1);
            } else if k1 < k2 {
                histogram.add_bucket(k1, v1);
                i += 1;
            } else {
                // Other histogram has a bucket not present in this histogram,
                // i.e., it is not a subset of this histogram
                return Err(Error::InvalidSubset);
            }
        }

        // Check that the subset histogram has been consumed
        if j < h.index.len() {
            return Err(Error::InvalidSubset);
        }

        // Fill remaining bucets, if any, from the superset histogram
        if i < self.index.len() {
            histogram.index.extend(&self.index[i..self.index.len()]);
            histogram.count.extend(&self.count[i..self.count.len()]);
        }

        Ok(histogram)
    }

    /// Return a single percentile from this histogram.
    ///
    /// The percentile should be in the inclusive range `0.0..=100.0`. For
    /// example, the 50th percentile (median) can be found using `50.0`.
    pub fn percentile(&self, percentile: f64) -> Result<Bucket, Error> {
        let total: u128 = self.count.iter().map(|v| *v as u128).sum();

        if !(0.0..=100.0).contains(&percentile) {
            return Err(Error::InvalidPercentile);
        }

        let search = ((total as f64) * percentile / 100.0).ceil() as usize;
        let mut seen: usize = 0;
        for (idx, count) in self.index.iter().zip(self.count.iter()) {
            seen += *count as usize;
            if seen >= search {
                return Ok(Bucket {
                    count: *count,
                    range: self.config.index_to_range(*idx),
                });
            }
        }

        // should never be reached; return highest bucket if not found
        let last_idx = self.index.len() - 1;
        Ok(Bucket {
            count: self.count[last_idx],
            range: self.config.index_to_range(self.index[last_idx]),
        })
    }

    /// Returns a new histogram with a reduced grouping power. The reduced
    /// grouping power should lie in the range (0..existing grouping power).
    ///
    /// This works by iterating over every bucket in the existing histogram
    /// and inserting the contained values into the new histogram. While we
    /// do not know the exact values of the data points (only that they lie
    /// within the bucket's range), it does not matter since the bucket is
    /// not split during downsampling and any value can be used.
    pub fn downsample(&self, grouping_power: u8) -> Result<SparseHistogram, Error> {
        if grouping_power >= self.config.grouping_power() {
            return Err(Error::MaxPowerTooLow);
        }

        let config = Config::new(grouping_power, self.config.max_value_power())?;
        let mut histogram = SparseHistogram::with_config(&config);

        // Multiple buckets in the old histogram will map to the same bucket
        // in the new histogram, so we have to aggregate bucket values from the
        // old histogram before inserting a bucket into the new downsampled
        // histogram. However, mappings between the histograms monotonically
        // increase, so once a bucket in the old histogram maps to a higher
        // bucket in the new histogram than is currently being aggregated,
        // the bucket can be sealed and inserted into the new histogram.
        let mut aggregating_idx: usize = 0;
        let mut aggregating_count: u64 = 0;
        for (idx, n) in self.index.iter().zip(self.count.iter()) {
            let new_idx = config.value_to_index(self.config.index_to_lower_bound(*idx))?;

            // If it maps to the currently aggregating bucket, merge counts
            if new_idx == aggregating_idx {
                aggregating_count += n;
                continue;
            }

            // Does not map to the aggregating bucket, so seal and store that bucket
            histogram.add_bucket(aggregating_idx, aggregating_count);

            // Start tracking this bucket as the current aggregating bucket
            aggregating_idx = new_idx;
            aggregating_count = *n;
        }

        // Add the final aggregated bucket
        histogram.add_bucket(aggregating_idx, aggregating_count);

        Ok(histogram)
    }
}

impl<'a> IntoIterator for &'a SparseHistogram {
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
    histogram: &'a SparseHistogram,
}

impl<'a> Iterator for Iter<'a> {
    type Item = Bucket;

    fn next(&mut self) -> Option<<Self as std::iter::Iterator>::Item> {
        if self.index >= self.histogram.index.len() {
            return None;
        }

        let bucket = Bucket {
            count: self.histogram.count[self.index],
            range: self
                .histogram
                .config
                .index_to_range(self.histogram.index[self.index]),
        };

        self.index += 1;

        Some(bucket)
    }
}

impl From<&Histogram> for SparseHistogram {
    fn from(histogram: &Histogram) -> Self {
        let mut index = Vec::new();
        let mut count = Vec::new();

        for (idx, n) in histogram.as_slice().iter().enumerate() {
            if *n > 0 {
                index.push(idx);
                count.push(*n);
            }
        }

        Self {
            config: histogram.config(),
            index,
            count,
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::Rng;
    use std::collections::HashMap;

    use super::*;
    use crate::standard::Histogram;

    #[test]
    fn wrapping_add() {
        let config = Config::new(7, 32).unwrap();

        let h1 = SparseHistogram {
            config,
            index: vec![1, 3, 5],
            count: vec![6, 12, 7],
        };

        let h2 = SparseHistogram::with_config(&config);

        let h3 = SparseHistogram {
            config,
            index: vec![2, 3, 6, 11, 13],
            count: vec![5, 7, 3, 15, 6],
        };

        let hdiff = SparseHistogram::new(6, 16).unwrap();
        let h = h1.wrapping_add(&hdiff);
        assert_eq!(h, Err(Error::IncompatibleParameters));

        let h = h1.wrapping_add(&h2).unwrap();
        assert_eq!(h.index, vec![1, 3, 5]);
        assert_eq!(h.count, vec![6, 12, 7]);

        let h = h2.wrapping_add(&h3).unwrap();
        assert_eq!(h.index, vec![2, 3, 6, 11, 13]);
        assert_eq!(h.count, vec![5, 7, 3, 15, 6]);

        let h = h1.wrapping_add(&h3).unwrap();
        assert_eq!(h.index, vec![1, 2, 3, 5, 6, 11, 13]);
        assert_eq!(h.count, vec![6, 5, 19, 7, 3, 15, 6]);
    }

    #[test]
    fn checked_sub() {
        let config = Config::new(7, 32).unwrap();

        let h1 = SparseHistogram {
            config,
            index: vec![1, 3, 5],
            count: vec![6, 12, 7],
        };

        let hparams = SparseHistogram::new(6, 16).unwrap();
        let h = h1.checked_sub(&hparams);
        assert_eq!(h, Err(Error::IncompatibleParameters));

        let hempty = SparseHistogram::with_config(&config);
        let h = h1.checked_sub(&hempty).unwrap();
        assert_eq!(h.index, vec![1, 3, 5]);
        assert_eq!(h.count, vec![6, 12, 7]);

        let hclone = h1.clone();
        let h = h1.checked_sub(&hclone).unwrap();
        assert!(h.index.is_empty());
        assert!(h.count.is_empty());

        let hlarger = SparseHistogram {
            config,
            index: vec![1, 3, 5],
            count: vec![4, 13, 7],
        };
        let h = h1.checked_sub(&hlarger);
        assert_eq!(h, Err(Error::Underflow));

        let hmore = SparseHistogram {
            config,
            index: vec![1, 5, 7],
            count: vec![4, 7, 1],
        };
        let h = h1.checked_sub(&hmore);
        assert_eq!(h, Err(Error::InvalidSubset));

        let hdiff = SparseHistogram {
            config,
            index: vec![1, 2, 5],
            count: vec![4, 1, 7],
        };
        let h = h1.checked_sub(&hdiff);
        assert_eq!(h, Err(Error::InvalidSubset));

        let hsubset = SparseHistogram {
            config,
            index: vec![1, 3],
            count: vec![5, 9],
        };
        let h = h1.checked_sub(&hsubset).unwrap();
        assert_eq!(h.index, vec![1, 3, 5]);
        assert_eq!(h.count, vec![1, 3, 7]);
    }

    #[test]
    fn percentile() {
        let mut hstandard = Histogram::new(4, 10).unwrap();
        for v in 1..1024 {
            let _ = hstandard.increment(v);
        }

        let hsparse = SparseHistogram::from(&hstandard);

        for percentile in [1.0, 10.0, 25.0, 50.0, 75.0, 90.0, 99.0, 99.9] {
            let bstandard = hstandard.percentile(percentile).unwrap();
            let bsparse = hsparse.percentile(percentile).unwrap();

            assert_eq!(bsparse, bstandard);
        }
    }

    fn compare_histograms(hstandard: &Histogram, hsparse: &SparseHistogram) {
        assert_eq!(hstandard.config(), hsparse.config);

        let mut buckets: HashMap<usize, u64> = HashMap::new();
        for (idx, count) in hsparse.index.iter().zip(hsparse.count.iter()) {
            let _ = buckets.insert(*idx, *count);
        }

        for (idx, count) in hstandard.as_slice().iter().enumerate() {
            if *count > 0 {
                let v = buckets.get(&idx).unwrap();
                assert_eq!(*v, *count);
            }
        }
    }

    #[test]
    fn snapshot() {
        let mut hstandard = Histogram::new(5, 10).unwrap();

        for v in 1..1024 {
            let _ = hstandard.increment(v);
        }

        // Convert to sparse and store buckets in a hash for random lookup
        let hsparse = SparseHistogram::from(&hstandard);
        compare_histograms(&hstandard, &hsparse);
    }

    #[test]
    fn downsample() {
        let mut histogram = Histogram::new(8, 32).unwrap();
        let mut rng = rand::thread_rng();

        // Generate 10,000 values to store in a sorted array and a histogram
        for _ in 0..10000 {
            let v: u64 = rng.gen_range(1..2_u64.pow(histogram.config.max_value_power() as u32));
            let _ = histogram.increment(v);
        }

        let hsparse = SparseHistogram::from(&histogram);
        compare_histograms(&histogram, &hsparse);

        // Downsample and compare heck the percentiles lie within error margin
        let grouping_power = histogram.config.grouping_power();
        for factor in 1..grouping_power {
            let reduced_gp = grouping_power - factor;
            let h1 = histogram.downsample(reduced_gp).unwrap();
            let h2 = hsparse.downsample(reduced_gp).unwrap();
            compare_histograms(&h1, &h2);
        }
    }
}
