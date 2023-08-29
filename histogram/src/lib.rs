//! This crate contains a collection of histogram data structures to help count
//! occurrences of values and report on their distribution.
//!
//! There are several implementations to choose from, with each targeting
//! a specific use-case.
//!
//! All the implementations share the same bucketing / binning strategy and
//! allow you to store values across a wide range with minimal loss of
//! precision. We do this by using linear buckets for the smaller values in the
//! histogram and transition to logarithmic buckets with linear subdivisions for
//! buckets that contain larger values. The indexing strategy is designed to be
//! efficient, allowing for blazingly fast increments.
//!
//! * [`Histogram`][`crate::Histogram`] - when a very fast histogram is all you
//!   need
//! * [`atomic::Histogram`][`crate::atomic::Histogram`] - a histogram with
//!   atomic operations
//! * [`sliding_window::Histogram`][`crate::sliding_window::Histogram`] - if you
//!   care about data points within a bounded range of time, with old values
//!   automatically dropping out
//! * [`sliding_window::atomic::Histogram`][`crate::sliding_window::atomic::Histogram`] -
//!   a sliding window histogram with atomic operations
//!
//! Additionally, there is a compact representation of a histogram which allows
//! for efficient serialization when the data is sparse:
//! * [`compact::Histogram`][`crate::compact::Histogram`] - a compact
//!   representation of a histogram for serialization when the data is sparse

pub mod atomic;
pub mod compact;
pub mod sliding_window;

mod bucket;
mod config;
mod errors;
mod standard;

pub use clocksource::precise::{Instant, UnixInstant};

pub use bucket::Bucket;
pub use errors::{BuildError, Error};
pub use standard::Histogram;

use crate::config::Config;
use clocksource::precise::{AtomicInstant, Duration};
use core::sync::atomic::Ordering;

/// A private trait that allows us to share logic across histogram types.
trait _Histograms {
    fn config(&self) -> Config;

    fn total_count(&self) -> u128;

    fn get_count(&self, index: usize) -> u64;

    fn get_bucket(&self, index: usize) -> Option<Bucket> {
        if index >= self.config().total_bins() {
            return None;
        }

        Some(Bucket {
            count: self.get_count(index),
            lower: self.config().index_to_lower_bound(index),
            upper: self.config().index_to_upper_bound(index),
        })
    }

    fn percentiles(&self, percentiles: &[f64]) -> Result<Vec<(f64, Bucket)>, Error> {
        // get the total count across all buckets
        let total: u128 = self.total_count();

        // if the histogram is empty, then we should return an error
        if total == 0_u128 {
            return Err(Error::Empty);
        }

        // sort the requested percentiles so we can find them in a single pass
        let mut percentiles = percentiles.to_vec();
        percentiles.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // get a sorted list of the counts needed for each percentile
        let counts: Vec<u128> = percentiles
            .iter()
            .map(|p| (p / 100.0 * total as f64).ceil() as u128)
            .collect();

        // a vec to hold the results
        let mut result = Vec::with_capacity(percentiles.len());

        let mut percentile_idx = 0;
        let mut bucket_idx = 0;
        let mut partial_sum = self.get_count(bucket_idx) as u128;

        while percentile_idx < percentiles.len() {
            let percentile = percentiles[percentile_idx];
            let count = counts[percentile_idx];

            // repeatedly push percentile-bucket pairs into the result while our
            // current partial sum fulfills the count needed for each percentile
            if partial_sum >= count {
                result.push((percentile, self.get_bucket(bucket_idx).unwrap()));
                percentile_idx += 1;
                continue;
            }

            // increment the partial sum by the count of the next bucket
            bucket_idx += 1;
            if bucket_idx >= self.config().total_bins() {
                break;
            }
            partial_sum += self.get_count(bucket_idx) as u128;
        }

        Ok(result)
    }

    fn percentile(&self, percentile: f64) -> Result<Bucket, Error> {
        self.percentiles(&[percentile])
            .map(|v| v.first().unwrap().1)
    }
}
