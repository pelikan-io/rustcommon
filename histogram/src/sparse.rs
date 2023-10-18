use crate::{Bucket, Config, Error, Snapshot};

/// This histogram is a sparse, columnar representation of the regular
/// Histogram. It is significantly smaller than a regular Histogram
/// when a large number of buckets are zero, which is a frequent
/// occurence. It stores an individual vector for each field
/// of non-zero buckets. Assuming index[0] = n, (index[0], count[0])
/// corresponds to the nth bucket.
#[derive(Debug, PartialEq)]
pub struct SparseHistogram {
    /// parameters representing the resolution and the range of
    /// the histogram tracking request latencies
    pub config: Config,
    /// total number of data points in the histogram
    pub total: u128,
    /// indices for the non-zero buckets in the histogram
    pub index: Vec<usize>,
    /// histogram bucket counts corresponding to the indices
    pub count: Vec<u64>,
}

impl SparseHistogram {
    fn add_bucket(&mut self, idx: usize, n: u64) {
        self.index.push(idx);
        self.count.push(n);
    }

    /// Merges two Histograms and returns the results in a new Histogram.
    ///
    /// Both histograms must have the same configuration parameters.
    /// Buckets which have values in both histograms are allowed to wrap.
    #[allow(clippy::comparison_chain)]
    pub fn merge(&self, h: &SparseHistogram) -> Result<SparseHistogram, Error> {
        if self.config != h.config {
            return Err(Error::IncompatibleParameters);
        }

        let mut histogram = SparseHistogram {
            config: self.config,
            total: self.total + h.total,
            index: Vec::new(),
            count: Vec::new(),
        };

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
            histogram.index.extend(&h.index[i..h.index.len()]);
            histogram.count.extend(&h.count[i..h.count.len()]);
        }

        Ok(histogram)
    }

    /// Return a single percentile from this histogram.
    ///
    /// The percentile should be in the inclusive range `0.0..=100.0`. For
    /// example, the 50th percentile (median) can be found using `50.0`.
    pub fn percentile(&self, percentile: f64) -> Result<Bucket, Error> {
        if self.total == 0 {
            return Err(Error::Empty);
        }

        if !(0.0..=100.0).contains(&percentile) {
            return Err(Error::InvalidPercentile);
        }

        let search = ((self.total as f64) * percentile / 100.0).ceil() as usize;
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

        // should be unreachable
        Err(Error::Unreachable)
    }
}

impl From<&Snapshot> for SparseHistogram {
    fn from(snapshot: &Snapshot) -> Self {
        let mut index = Vec::new();
        let mut count = Vec::new();
        let mut total: u128 = 0;

        for (i, bucket) in snapshot
            .into_iter()
            .enumerate()
            .filter(|(_i, bucket)| bucket.count() != 0)
        {
            index.push(i);
            count.push(bucket.count());
            total += bucket.count() as u128;
        }

        Self {
            config: snapshot.config(),
            total,
            index,
            count,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::standard::Histogram;

    #[test]
    fn merge() {
        let config = Config::new(7, 32).unwrap();

        let h1 = SparseHistogram {
            config,
            total: 25,
            index: vec![1, 3, 5],
            count: vec![6, 12, 7],
        };

        let h2 = SparseHistogram {
            config,
            total: 0,
            index: Vec::new(),
            count: Vec::new(),
        };

        let h3 = SparseHistogram {
            config,
            total: 30,
            index: vec![2, 3, 4, 11],
            count: vec![5, 7, 3, 15],
        };

        let hdiff = SparseHistogram {
            config: Config::new(6, 16).unwrap(),
            total: 0,
            index: Vec::new(),
            count: Vec::new(),
        };

        let h = h1.merge(&hdiff);
        assert_eq!(h, Err(Error::IncompatibleParameters));

        let h = h1.merge(&h2).unwrap();
        assert_eq!(h.total, 25);
        assert_eq!(h.index, vec![1, 3, 5]);
        assert_eq!(h.count, vec![6, 12, 7]);

        let h = h2.merge(&h3).unwrap();
        assert_eq!(h.total, 30);
        assert_eq!(h.index, vec![2, 3, 4, 11]);
        assert_eq!(h.count, vec![5, 7, 3, 15]);

        let h = h1.merge(&h3).unwrap();
        assert_eq!(h.total, 55);
        assert_eq!(h.index, vec![1, 2, 3, 4, 5, 11]);
        assert_eq!(h.count, vec![6, 5, 19, 3, 7, 15]);
    }

    #[test]
    fn percentile() {
        let mut hstandard = Histogram::new(4, 10).unwrap();
        for v in 1..1024 {
            let _ = hstandard.increment(v);
        }

        let hsparse = SparseHistogram::from(&hstandard.snapshot());

        for percentile in [1.0, 10.0, 25.0, 50.0, 75.0, 90.0, 99.0, 99.9] {
            let bstandard = hstandard.percentile(percentile).unwrap();
            let bsparse = hsparse.percentile(percentile).unwrap();

            assert_eq!(bsparse, bstandard);
        }
    }

    #[test]
    fn snapshot() {
        let mut hstandard = Histogram::new(5, 10).unwrap();

        for v in 1..1024 {
            let _ = hstandard.increment(v);
        }

        // Convert to sparse and store buckets in a hash for random lookup
        let hsparse = SparseHistogram::from(&hstandard.snapshot());
        let mut buckets: HashMap<usize, u64> = HashMap::new();
        for (idx, count) in hsparse.index.iter().zip(hsparse.count.iter()) {
            let _ = buckets.insert(*idx, *count);
        }

        for (idx, count) in hstandard.as_slice().iter().enumerate() {
            if *count != 0 {
                let v = buckets.get(&idx).unwrap();
                assert_eq!(*v, *count);
            }
        }
    }
}
