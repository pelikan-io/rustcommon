use crate::{Bucket, Config, Error, Histogram};
use std::time::SystemTime;

/// A snapshot of a histogram across a time range.
pub struct Snapshot {
    // note: `Histogram` contains the start time
    pub(crate) end: SystemTime,
    pub(crate) histogram: Histogram,
}

impl Snapshot {
    /// Return the time range of the snapshot.
    pub fn range(&self) -> core::ops::Range<SystemTime> {
        self.histogram.start..self.end
    }

    /// Return a collection of percentiles from this snapshot.
    ///
    /// Each percentile should be in the inclusive range `0.0..=100.0`. For
    /// example, the 50th percentile (median) can be found using `50.0`.
    ///
    /// The results will be sorted by the percentile.
    pub fn percentiles(&self, percentiles: &[f64]) -> Result<Vec<(f64, Bucket)>, Error> {
        self.histogram.percentiles(percentiles)
    }

    /// Return a single percentile from this snapshot.
    ///
    /// The percentile should be in the inclusive range `0.0..=100.0`. For
    /// example, the 50th percentile (median) can be found using `50.0`.
    pub fn percentile(&self, percentile: f64) -> Result<Bucket, Error> {
        self.histogram.percentile(percentile)
    }

    /// Merges two snapshots which cover the same time range.
    ///
    /// An error is raised on overflow.
    pub fn checked_merge(&self, rhs: &Self) -> Result<Self, Error> {
        if self.range() != rhs.range() {
            return Err(Error::IncompatibleTimeRange);
        }

        let histogram = self.histogram.checked_add(&rhs.histogram)?;

        Ok(Self {
            end: rhs.end,
            histogram,
        })
    }

    /// Appends the provided snapshot onto this snapshot, extending the covered
    /// time range and combining the bucket counts.
    ///
    /// An error is raised on overflow.
    pub fn checked_add(&self, rhs: &Self) -> Result<Self, Error> {
        if self.end != rhs.histogram.start {
            return Err(Error::IncompatibleTimeRange);
        }

        let histogram = self.histogram.checked_add(&rhs.histogram)?;

        Ok(Self {
            end: rhs.end,
            histogram,
        })
    }

    /// Appends the provided snapshot onto this snapshot, extending the covered
    /// time range and combining the bucket counts.
    ///
    /// Bucket counters will wrap on overflow.
    pub fn wrapping_add(&self, rhs: &Self) -> Result<Self, Error> {
        if self.end != rhs.histogram.start {
            return Err(Error::IncompatibleTimeRange);
        }

        let histogram = self.histogram.wrapping_add(&rhs.histogram)?;

        Ok(Self {
            end: rhs.end,
            histogram,
        })
    }

    /// Appends the provided snapshot onto this snapshot, shrinking the covered
    /// time range and producing a delta of the bucket counts.
    ///
    /// An error is raised on overflow.
    pub fn checked_sub(&self, rhs: &Self) -> Result<Self, Error> {
        if self.histogram.start < rhs.histogram.start {
            return Err(Error::IncompatibleTimeRange);
        }

        if self.end < rhs.end {
            return Err(Error::IncompatibleTimeRange);
        }

        let mut histogram = self.histogram.checked_sub(&rhs.histogram)?;

        histogram.start = rhs.end;

        Ok(Self {
            end: self.end,
            histogram,
        })
    }

    /// Appends the provided snapshot onto this snapshot, extending the covered
    /// time range and combining the bucket counts.
    ///
    /// Bucket counters will wrap on overflow.
    pub fn wrapping_sub(&self, rhs: &Self) -> Result<Self, Error> {
        if self.histogram.start != rhs.histogram.start {
            return Err(Error::IncompatibleTimeRange);
        }

        if self.end < rhs.end {
            return Err(Error::IncompatibleTimeRange);
        }

        let mut histogram = self.histogram.wrapping_sub(&rhs.histogram)?;

        histogram.start = rhs.end;

        Ok(Self {
            end: self.end,
            histogram,
        })
    }

    /// Returns the bucket configuration of the snapshot.
    pub fn config(&self) -> Config {
        self.histogram.config()
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
