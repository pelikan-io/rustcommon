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
//! Please see: <https://h2histogram.org>

mod atomic;
mod bucket;
mod config;
mod errors;
mod sparse;
mod standard;

pub use atomic::AtomicHistogram;
pub use bucket::Bucket;
pub use config::Config;
pub use errors::Error;
pub use sparse::SparseHistogram;
pub use standard::Histogram;
