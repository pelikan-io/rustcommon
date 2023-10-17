// use serde::{Deserialize, Serialize};

use crate::{Error, Snapshot};

/// This histogram is a sparse, columnar representation of the regular
/// Histogram. It is significantly smaller than a regular Histogram
/// when a large number of buckets are zero, which is a frequent
/// occurence. It stores an individual vector for each field
/// of non-zero buckets. Assuming index[0] = n, (index[0], count[0])
/// corresponds to the nth bucket.
// #[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[derive(Debug, Default, PartialEq)]
pub struct SparseHistogram {
    /// parameters representing the resolution and the range of
    /// the histogram tracking request latencies
    pub grouping_power: u8,
    pub max_value_power: u8,
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
        if self.grouping_power != h.grouping_power || self.max_value_power != h.max_value_power {
            return Err(Error::IncompatibleParameters);
        }

        let mut histogram = SparseHistogram {
            grouping_power: self.grouping_power,
            max_value_power: self.max_value_power,
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
}

impl From<&Snapshot> for SparseHistogram {
    fn from(snapshot: &Snapshot) -> Self {
        let mut index = Vec::new();
        let mut count = Vec::new();

        for (i, bucket) in snapshot
            .into_iter()
            .enumerate()
            .filter(|(_i, bucket)| bucket.count() != 0)
        {
            index.push(i);
            count.push(bucket.count());
        }

        let config = snapshot.config();
        Self {
            grouping_power: config.grouping_power(),
            max_value_power: config.max_value_power(),
            index,
            count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge() {
        let h1 = SparseHistogram {
            grouping_power: 8,
            max_value_power: 32,
            index: vec![1, 3, 5],
            count: vec![6, 12, 7],
        };

        let h2 = SparseHistogram {
            grouping_power: 8,
            max_value_power: 32,
            index: Vec::new(),
            count: Vec::new(),
        };

        let h3 = SparseHistogram {
            grouping_power: 8,
            max_value_power: 32,
            index: vec![2, 3, 4, 11],
            count: vec![5, 7, 3, 15],
        };

        let h = h1.merge(&SparseHistogram::default());
        assert_eq!(h, Err(Error::IncompatibleParameters));

        let h = h1.merge(&h2).unwrap();
        assert_eq!(h.index, vec![1, 3, 5]);
        assert_eq!(h.count, vec![6, 12, 7]);

        let h = h2.merge(&h3).unwrap();
        assert_eq!(h.index, vec![2, 3, 4, 11]);
        assert_eq!(h.count, vec![5, 7, 3, 15]);

        let h = h1.merge(&h3).unwrap();
        assert_eq!(h.index, vec![1, 2, 3, 4, 5, 11]);
        assert_eq!(h.count, vec![6, 5, 19, 3, 7, 15]);
    }
}
