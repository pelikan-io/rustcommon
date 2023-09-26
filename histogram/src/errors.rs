use thiserror::Error;

/// Errors returned while constructing a histogram.
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum BuildError {
    #[error("max power is too high, check that n <= 64")]
    MaxPowerTooHigh,
    #[error("max power is too low, check that a + b < n")]
    MaxPowerTooLow,
    #[error("boxed slice length does not match the config")]
    FromRawWrongLength,
    #[error("sliding window interval cannot be greater than 1 hour")]
    IntervalTooLong,
    #[error("sliding window interval cannot be less than than 1 millisecond")]
    IntervalTooShort,
}

/// Errors returned for operations on histograms.
#[non_exhaustive]
#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("histogram contains no observations")]
    Empty,
    #[error("invalid percentile, must be in range 0.0..=100.0")]
    InvalidPercentile,
    #[error("the value is outside of the storable range")]
    OutOfRange,
    #[error("the value is outside of the sliding window")]
    OutOfSlidingWindow,
    #[error("the histogram parameters are incompatible")]
    IncompatibleParameters,
    #[error("the snapshot time ranges do not allow this operation")]
    IncompatibleTimeRange,
    #[error("an overflow occurred")]
    Overflow,
}
