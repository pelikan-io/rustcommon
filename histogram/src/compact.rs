use serde_derive::{Deserialize, Serialize};

use crate::histogram::Histogram;

/// A `CompactHistogram` is a sparse, columnar representation of the
/// Histogram. It is significantly smaller than a regular Histogram
/// when a large number of buckets are zero, which is a frequent
/// occurence; consequently it is used as the serialization format
/// of the Histogram. It stores an individual vector for each field
/// of non-zero buckets. Assuming index[0] = n, (index[0], count[0])
/// corresponds to the nth bucket.
#[derive(Serialize, Deserialize)]
pub struct CompactHistogram {
    /// parameters representing the resolution and the range of
    /// the histogram tracking request latencies
    pub m: u32,
    pub r: u32,
    pub n: u32,
    /// indices for the non-zero buckets in the histogram
    pub index: Vec<usize>,
    /// histogram bucket counts corresponding to the indices
    pub count: Vec<u32>,
}

impl CompactHistogram {
    pub fn new() -> Self {
        Self {
            m: 0,
            r: 0,
            n: 0,
            index: Vec::new(),
            count: Vec::new(),
        }
    }
}

impl Default for CompactHistogram {
    fn default() -> Self {
        Self::new()
    }
}

impl From<&Histogram> for CompactHistogram {
    fn from(histogram: &Histogram) -> Self {
        let mut index = Vec::new();
        let mut count = Vec::new();

        for (i, bucket) in histogram
            .into_iter()
            .enumerate()
            .filter(|(_i, bucket)| bucket.count() != 0)
        {
            index.push(i);
            count.push(bucket.count());
        }

        let p = histogram.parameters();
        Self {
            m: p.0,
            r: p.1,
            n: p.2,
            index,
            count,
        }
    }
}
