use crate::{Bucket, Histogram};
use std::time::SystemTime;

/// An immutable snapshot of a histogram at a point in time.
pub struct Snapshot {
    pub(crate) created_at: SystemTime,
    pub(crate) histogram: Histogram,
}

impl Snapshot {
    /// Get a reference to the histogram.
    pub fn histogram(&self) -> &Histogram {
        &self.histogram
    }

    /// Returns the instant this snapshot was created.
    pub fn created_at(&self) -> SystemTime {
        self.created_at
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
