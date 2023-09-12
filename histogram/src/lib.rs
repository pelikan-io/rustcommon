//! This crate contains the implementations of a base-2 bucketed histogram as
//! either a free-running histogram, or as a sliding window histogram.
//!
//! The free-running histogram is implemented without atomics and can be used in
//! cases where a single threaded access to the histogram is desired. Typically,
//! this type would be used when you want latched histograms or want a single
//! histogram to cover a full data set.
//!
//! A sliding window histogram maintains counts for quantized values across a
//! window of time. They are useful for metrics use-cases where you wish to
//! report on the distribution of values across some recent time period. For
//! example a latency distribution for all operations in the past minute. At its
//! core, this implementation is a ring buffer of histograms where each
//! histogram stores a snapshot of the live histogram at some point in time.
//! Each of these histograms uses a base-2 bucketing strategy with a linear and
//! logarithmic region. The linear region contains buckets that are fixed-width.
//! The logarithmic region contains segments that span powers of two. Each
//! logarithmic segment is sub-divided linearly into some number of buckets.

mod bucket;
mod config;
mod errors;
mod sliding_window;
mod snapshot;
mod standard;

pub use clocksource::precise::{Instant, UnixInstant};

pub use bucket::Bucket;
pub use errors::{BuildError, Error};
pub use sliding_window::{Builder as SlidingWindowBuilder, Histogram as SlidingWindowHistogram};
pub use snapshot::Snapshot;
pub use standard::Histogram;

use crate::config::Config;
use clocksource::precise::{AtomicInstant, Duration};
use core::ops::{Range, RangeInclusive};
use core::sync::atomic::Ordering;
