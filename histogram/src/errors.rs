use thiserror::Error;

/// Errors returned while constructing a histogram.
#[derive(Error, Debug)]
pub enum BuildError {
    #[error("max power is too high, check that n <= 64")]
    MaxPowerTooHigh,
    #[error("max power is too low, check that a + b < n")]
    MaxPowerTooLow,
    #[error("boxed slice length does not match the config")]
    FromRawWrongLength,
}

/// Errors returned for operations on histograms.
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
}
