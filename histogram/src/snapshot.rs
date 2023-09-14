//! A snapshot captures the distribution across a time range.

use crate::{Bucket, Error, Histogram, Range, UnixInstant};

/// An immutable snapshot of a distribution for a fixed time window.
pub struct Snapshot {
    pub(crate) range: Range<UnixInstant>,
    pub(crate) histogram: Histogram,
}

impl Snapshot {
    /// Get a reference to the raw counters.
    pub fn as_slice(&self) -> &[u64] {
        &self.histogram.buckets
    }

    /// Return a collection of percentiles from this histogram.
    ///
    /// Each percentile should be in the inclusive range `0.0..=100.0`. For
    /// example, the 50th percentile (median) can be found using `50.0`.
    ///
    /// The results will be sorted by the percentile.
    pub fn percentiles(&self, percentiles: &[f64]) -> Result<Vec<(f64, Bucket)>, Error> {
        self.histogram.percentiles(percentiles)
    }

    /// Return a single percentile from this histogram.
    ///
    /// The percentile should be in the inclusive range `0.0..=100.0`. For
    /// example, the 50th percentile (median) can be found using `50.0`.
    pub fn percentile(&self, percentile: f64) -> Result<Bucket, Error> {
        self.histogram.percentile(percentile)
    }

    /// Returns the time range covered by this snapshot.
    pub fn range(&self) -> Range<UnixInstant> {
        self.range.clone()
    }

    /// Returns the inclusive lower bound for the snapshot.
    pub fn start(&self) -> UnixInstant {
        self.range.start
    }

    /// Returns the exclusive upper bound for the snapshot.
    pub fn end(&self) -> UnixInstant {
        self.range.end
    }
}

impl<'a> IntoIterator for &'a Snapshot {
    type Item = Bucket;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Iter {
            iter: self.histogram.into_iter(),
        }
    }
}

/// An iterator across the histogram buckets.
pub struct Iter<'a> {
    iter: crate::standard::Iter<'a>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = Bucket;

    fn next(&mut self) -> Option<<Self as std::iter::Iterator>::Item> {
        self.iter.next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size() {
        assert_eq!(std::mem::size_of::<Snapshot>(), 80);
    }
}
