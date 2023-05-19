use core::sync::atomic::{AtomicU32, Ordering};

// A simple concurrent histogram that can be used to track the distribution of
// occurances of u64 values. Internally it uses 32bit atomic counters.
pub struct Histogram {
    pub(crate) buckets: Box<[AtomicU32]>,
    pub(crate) max: u64,
    pub(crate) a: u32,
    pub(crate) b: u32,
    pub(crate) cutoff_value: u64,
    pub(crate) cutoff_power: usize,
    pub(crate) lower_bin_count: usize,
    pub(crate) upper_bin_divisions: usize,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Bucket {
    lower: u64,
    upper: u64,
}

impl Histogram {
    /// # Panics
    /// This function will panic if:
    /// * `n` is greater than 64
    /// * `n` is not greater than `a + b`
    pub fn new(a: u8, b: u8, n: u8) -> Self {
        let a: u32 = a.into();
        let b: u32 = b.into();
        let n: u32 = n.into();

        // we only allow values up to 2^64
        assert!(n <= 64);

        // check that the other parameters make sense together
        assert!(a + b < n);

        let cutoff_power = a + b + 1;
        let cutoff_value = 2_u64.pow(cutoff_power);
        let lower_bin_width = 2_usize.pow(a);
        let upper_bin_divisions = 2_usize.pow(b);

        let max = if n == 64 {
            u64::MAX
        } else {
            2_u64.pow(n)
        };

        let lower_bin_count = (cutoff_value / lower_bin_width as u64) as usize;
        let upper_bin_count = (n - (a + b + 1)) as usize * upper_bin_divisions;
        let total_bins = lower_bin_count + upper_bin_count;

        let mut buckets = Vec::with_capacity(total_bins);
        buckets.resize_with(total_bins, || { AtomicU32::new(0) });

        Self {
            buckets: buckets.into(),
            max,
            a,
            b,
            cutoff_power: cutoff_power as usize,
            cutoff_value,
            lower_bin_count,
            upper_bin_divisions,
        }
    }

    /// Provides raw access to the bucket counts.
    pub fn as_raw(&self) -> &[AtomicU32] {
        &self.buckets
    }

    /// # Panics
    /// This function will panic if the value is larger than the max configured
    /// value for this histogram.
    fn value_to_index(&self, value: u64) -> usize {
        if value < self.cutoff_value {
            return (value >> self.a) as usize;
        }

        if value > self.max {
            panic!("out of range");
        }

        let power = (63 - value.leading_zeros()) as usize;
        let log_bin = power - self.cutoff_power;
        let offset = (value - (1 << power)) >> (power - self.b as usize);

        self.lower_bin_count + log_bin * self.upper_bin_divisions + offset as usize
    }

    fn index_to_lower_bound(&self, index: usize) -> u64 {
        let a = self.a as u64;
        let b = self.b as u64;
        let g = index as u64 >> self.b;
        let h = index as u64 - g * (1 << self.b);

        if g < 1 {
            (1 << a) * h
        } else {
            (1 << (a + b + g - 1)) + (1 << (a + g - 1)) * h
        }
    }

    fn index_to_upper_bound(&self, index: usize) -> u64 {
        if index == self.buckets.len() - 1 {
            return self.max;
        }

        let a = self.a as u64;
        let b = self.b as u64;
        let g = index as u64 >> self.b;
        let h = index as u64 - g * (1 << self.b) + 1;

        if g < 1 {
            (1 << a) * h - 1
        } else {
            (1 << (a + b + g - 1)) + (1 << (a + g - 1)) * h - 1
        }
    }

    fn get_bucket(&self, index: usize) -> Bucket {
        Bucket {
            lower: self.index_to_lower_bound(index),
            upper: self.index_to_upper_bound(index),
        }
    }

    /// Increment the count of observations for the bucket corresponding to the
    /// provided value by one.
    ///
    /// # Caution
    /// Wrapping addition is performed and the internal counters are only 32bit.
    /// If an increment causes wrapping and a percentile is requested from this
    /// histogram, you may get incorrect results.
    ///
    /// However, when used in combination with the Snapshots
    pub fn increment(&self, value: u64) {
        let index = self.value_to_index(value);
        self.buckets[index].fetch_add(1, Ordering::Relaxed);
    }

    pub fn percentile(&self, percentile: f64) -> Option<Bucket> {
        self.percentiles(&[percentile]).map(|v| v.first().unwrap().1)
    }

    pub fn percentiles(&self, percentiles: &[f64]) -> Option<Vec<(f64, Bucket)>> {
        // get the total count across all buckets as a u64
        let total: u64 = self.buckets.iter().map(|v| v.load(Ordering::Relaxed) as u64).sum();

        // if the histogram is empty, then we should return an error
        if total == 0 {
            // TODO(brian): this should return an error =)
            return None;
        }

        // sort the requested percentiles so we can find them in a single pass
        let mut percentiles = percentiles.to_vec();
        percentiles.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mut result = Vec::new();

        let mut have = 0_u64;
        let mut percentile_idx = 0_usize;
        let mut current_idx = 0_usize;
        let mut max_idx = 0_usize;

        // outer loop walks through the requested percentiles
        'outer: loop {
            // if we have all the requested percentiles, return the result
            if percentile_idx >= percentiles.len() {
                return Some(result);
            }

            // calculate the count we need to have for the requested percentile
            let percentile = percentiles[percentile_idx];
            let needed = (percentile / 100.0 * total as f64).ceil() as u64;

            // if the count is already that high, push to the results and
            // continue onto the next percentile
            if have >= needed {
                result.push((percentile, self.get_bucket(current_idx)));
                percentile_idx += 1;
                continue;
            }

            // the inner loop walks through the buckets
            'inner: loop {
                // if we've run out of buckets, break the outer loop
                if current_idx >= self.buckets.len() {
                    break 'outer;
                }

                // get the current count for the current bucket
                let current_count = self.buckets[current_idx].load(Ordering::Relaxed);

                // track the highest index with a non-zero count
                if current_count > 0 {
                    max_idx = current_idx;
                }

                // increment what we have by the current bucket count
                have += current_count as u64;

                // if this is enough for the requested percentile, push to the
                // results and break the inner loop to move onto the next
                // percentile
                if have >= needed {
                    result.push((percentile, self.get_bucket(current_idx)));
                    percentile_idx += 1;
                    current_idx += 1;
                    break 'inner;
                }

                // increment the current_idx so we continue from the next bucket
                current_idx += 1;
            }
        }

        // fill the remaining percentiles with the highest non-zero bucket's
        // value. this is possible if the histogram has been modified while we
        // are still iterating. 
        for percentile in percentiles.iter().skip(result.len()) {
            result.push((*percentile, self.get_bucket(max_idx)));
        }

        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // Test that the number of buckets matches the expected count
    fn bucket_counts() {
        let histogram = Histogram::new(0, 2, 64);
        assert_eq!(histogram.buckets.len(), 252);

        let histogram = Histogram::new(0, 7, 64);
        assert_eq!(histogram.buckets.len(), 7424);

        let histogram = Histogram::new(0, 14, 64);
        assert_eq!(histogram.buckets.len(), 835_584);

        let histogram = Histogram::new(1, 2, 64);
        assert_eq!(histogram.buckets.len(), 248);

        let histogram = Histogram::new(8, 2, 64);
        assert_eq!(histogram.buckets.len(), 220);

        let histogram = Histogram::new(0, 2, 4);
        assert_eq!(histogram.buckets.len(), 12);
    }

    #[test]
    // Test value to index conversions
    fn value_to_idx() {
        let histogram = Histogram::new(0, 7, 64);
        assert_eq!(histogram.value_to_index(0), 0);
        assert_eq!(histogram.value_to_index(1), 1);
        assert_eq!(histogram.value_to_index(256), 256);
        assert_eq!(histogram.value_to_index(257), 256);
        assert_eq!(histogram.value_to_index(258), 257);
        assert_eq!(histogram.value_to_index(512), 384);
        assert_eq!(histogram.value_to_index(515), 384);
        assert_eq!(histogram.value_to_index(516), 385);
        assert_eq!(histogram.value_to_index(1024), 512);
        assert_eq!(histogram.value_to_index(1031), 512);
        assert_eq!(histogram.value_to_index(1032), 513);
        assert_eq!(histogram.value_to_index(u64::MAX - 1), 7423);
        assert_eq!(histogram.value_to_index(u64::MAX), 7423);
    }

    #[test]
    // Test index to lower bound conversion
    fn idx_to_lower_bound() {
        let histogram = Histogram::new(0, 7, 64);
        assert_eq!(histogram.index_to_lower_bound(0), 0);
        assert_eq!(histogram.index_to_lower_bound(1), 1);
        assert_eq!(histogram.index_to_lower_bound(256), 256);
        assert_eq!(histogram.index_to_lower_bound(384), 512);
        assert_eq!(histogram.index_to_lower_bound(512), 1024);
        assert_eq!(histogram.index_to_lower_bound(7423), 18_374_686_479_671_623_680);
    }

    #[test]
    // Test index to upper bound conversion
    fn idx_to_upper_bound() {
        let histogram = Histogram::new(0, 7, 64);
        assert_eq!(histogram.index_to_upper_bound(0), 0);
        assert_eq!(histogram.index_to_upper_bound(1), 1);
        assert_eq!(histogram.index_to_upper_bound(256), 257);
        assert_eq!(histogram.index_to_upper_bound(384), 515);
        assert_eq!(histogram.index_to_upper_bound(512), 1031);
        assert_eq!(histogram.index_to_upper_bound(7423), u64::MAX);
    }

    #[test]
    // Tests percentiles
    fn percentiles() {
        let histogram = Histogram::new(0, 7, 64);
        for i in 0..=100 {
            println!("increment: {i}");
            histogram.increment(i);
            assert_eq!(histogram.percentile(0.0), Some(Bucket { lower: 0, upper: 0 }));
            assert_eq!(histogram.percentile(100.0), Some(Bucket { lower: i, upper: i }));
        } 
        assert_eq!(histogram.percentile(25.0).map(|b| b.upper), Some(25));
        assert_eq!(histogram.percentile(50.0).map(|b| b.upper), Some(50));
        assert_eq!(histogram.percentile(75.0).map(|b| b.upper), Some(75));
        assert_eq!(histogram.percentile(90.0).map(|b| b.upper), Some(90));
        assert_eq!(histogram.percentile(99.0).map(|b| b.upper), Some(99));
        assert_eq!(histogram.percentile(99.9).map(|b| b.upper), Some(100));

        let percentiles: Vec<(f64, u64)> = histogram.percentiles(&[50.0, 90.0, 99.0, 99.9]).unwrap()
            .iter().map(|(p, b)| (*p, b.upper)).collect();

        assert_eq!(percentiles,
            vec![(50.0, 50), (90.0, 90), (99.0, 99), (99.9, 100)]
        );

        histogram.increment(1024);
        assert_eq!(histogram.percentile(99.9), Some(Bucket { lower: 1024, upper: 1031 }));
    }
}
