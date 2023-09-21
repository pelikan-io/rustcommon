//! This crate provides histogram implementations that are conceptually similar
//! to HdrHistogram, with modifications to the bucket construction and indexing
//! algorithms that we believe provide a simpler implementation and more
//! efficient runtime compared to the reference implementation of HdrHistogram.
//!
//! # Goals
//! * simple implementation
//! * fine-grained configuration
//! * efficient runtime
//!
//! # Background
//! Please see: <https://observablehq.com/@iopsystems/h2histogram>

mod bucket;
mod config;
mod errors;
mod parameters;
mod sliding_window;
mod snapshot;
mod standard;

pub use clocksource::precise::{Duration, Instant, UnixInstant};

pub use bucket::Bucket;
pub use errors::{BuildError, Error};
pub use parameters::Parameters;
pub use sliding_window::{Builder as SlidingWindowBuilder, Histogram as SlidingWindowHistogram};
pub use snapshot::Snapshot;
pub use standard::Histogram;

use crate::config::Config;
use clocksource::precise::AtomicInstant;
use core::ops::{Range, RangeInclusive};
use core::sync::atomic::Ordering;
