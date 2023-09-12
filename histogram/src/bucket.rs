//! Buckets represent quantized value ranges and a count of observations within
//! that range.

use crate::RangeInclusive;

/// A bucket represents a quantized range of values and a count of observations
/// that fall into that range.
#[derive(Clone, Debug, PartialEq)]
pub struct Bucket {
    pub(crate) count: u64,
    pub(crate) range: RangeInclusive<u64>,
}

impl Bucket {
    /// Returns the number of observations within the bucket's range.
    pub fn count(&self) -> u64 {
        self.count
    }

    /// Returns the range for the bucket.
    pub fn range(&self) -> RangeInclusive<u64> {
        self.range.clone()
    }

    /// Returns the inclusive lower bound for the bucket.
    pub fn start(&self) -> u64 {
        *self.range.start()
    }

    /// Returns the inclusive upper bound for the bucket.
    pub fn end(&self) -> u64 {
        *self.range.end()
    }
}
