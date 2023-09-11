//! This crate contains the implementation of a sliding window histogram.
//!
//! A sliding window histogram maintains counts for quantized values across a
//! window of time. They are useful for metrics use-cases where you wish to
//! report on the distribution of values across some recent time period. For
//! example a latency distribution for all operations in the past minute.
//!
//! At its core, this implementation is a ring buffer of histograms where each
//! histogram stores a snapshot of the live histogram at some point in time.
//! Each of these histograms uses a base-2 bucketing strategy with a linear and
//! logarithmic region. The linear region contains buckets that are fixed-width.
//! The logarithmic region contains segments that span powers of two. Each
//! logarithmic segment is sub-divided linearly into some number of buckets.

// mod builder;

// pub use builder::Builder;

// mod atomic;
// pub mod compact;
pub mod sliding_window;

mod bucket;
mod config;
mod errors;
mod snapshot;
mod standard;

pub use clocksource::precise::{Instant, UnixInstant};

pub use bucket::Bucket;
pub use errors::{BuildError, Error};
pub use snapshot::Snapshot;
pub use standard::Histogram;

use crate::config::Config;
use clocksource::precise::{AtomicInstant, Duration};
use core::sync::atomic::Ordering;

// /// A private trait that allows us to share logic across histogram types.
// trait _Histograms {
//     fn config(&self) -> Config;

//     fn total_count(&self) -> u128;

//     fn get_count(&self, index: usize) -> u64;

//     fn get_bucket(&self, index: usize) -> Option<Bucket> {
//         if index >= self.config().total_bins() {
//             return None;
//         }

//         Some(Bucket {
//             count: self.get_count(index),
//             lower: self.config().index_to_lower_bound(index),
//             upper: self.config().index_to_upper_bound(index),
//         })
//     }

//     fn percentiles(&self, percentiles: &[f64]) -> Result<Vec<(f64, Bucket)>, Error> {
//         // check that all percentiles are valid before doing any real work
//         for percentile in percentiles {
//             if *percentile < 0.0 || *percentile > 100.0 {
//                 return Err(Error::InvalidPercentile);
//             }
//         }

//         // get the total count across all buckets
//         let total: u128 = self.total_count();

//         // if the histogram is empty, then we should return an error
//         if total == 0_u128 {
//             return Err(Error::Empty);
//         }

//         // sort the requested percentiles so we can find them in a single pass
//         let mut percentiles = percentiles.to_vec();
//         percentiles.sort_by(|a, b| a.partial_cmp(b).unwrap());

//         let mut bucket_idx = 0;
//         let mut partial_sum = self.get_count(bucket_idx) as u128;

//         let result = percentiles.iter().filter_map(|percentile| {
//             let count = (percentile / 100.0 * total as f64).ceil() as u128;

//             while bucket_idx < (self.config().total_bins() - 1) {
//                 // found the matching bucket index for this percentile
//                 if partial_sum >= count {
//                     return Some((*percentile, self.get_bucket(bucket_idx).unwrap()));
//                 }

//                 // otherwise, increment the bucket index, partial sum, and loop
//                 bucket_idx += 1;
//                 partial_sum += self.get_count(bucket_idx) as u128;
//             }

//             None
//         }).collect();

//         Ok(result)
//     }

//     fn percentile(&self, percentile: f64) -> Result<Bucket, Error> {
//         self.percentiles(&[percentile])
//             .map(|v| v.first().unwrap().1)
//     }
// }
