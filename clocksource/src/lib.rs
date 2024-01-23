//! This crate provides time and duration types with fixed-size representations
//! with either coarse or precise resolution. This allows for using these types
//! in places where a known size is required.
//!
//! The internal representations also trade decreased representable ranges in
//! exchange for smaller types. This makes them appealing for short timescales
//! when smaller sized types are beneficial, such as in item metadata.
//!
//! Since the internal representations use a single 32bit or 64bit value, math
//! operations on the types are cheaper than they are with the standard time
//! types.

pub mod coarse;
pub mod datetime;
pub mod precise;

mod sys;

const MILLIS_PER_SEC: u64 = 1_000;
const MICROS_PER_SEC: u64 = 1_000_000;
const NANOS_PER_SEC: u64 = 1_000_000_000;
