//! Buckets represent quantized value ranges and a count of observations within
//! that range.

/// A bucket represents a quantized range of values and a count of observations
/// that fall into that range.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Bucket {
    pub(crate) count: u64,
    pub(crate) lower: u64,
    pub(crate) upper: u64,
}

impl Bucket {
    /// Returns the number of observations within the bucket's range.
    pub fn count(&self) -> u64 {
        self.count
    }

    /// Returns the range for the bucket.
    pub fn range(&self) -> std::ops::RangeInclusive<u64> {
        std::ops::RangeInclusive::new(self.lower, self.upper)
    }

    /// Returns the inclusive lower bound for the bucket.
    pub fn lower(&self) -> u64 {
        self.lower
    }

    /// Returns the inclusive upper bound for the bucket.
    pub fn upper(&self) -> u64 {
        self.upper
    }
}
