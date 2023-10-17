use thiserror::Error;

/// Errors returned for histogram construction and operations.
#[non_exhaustive]
#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("max power is too high, check that n <= 64")]
    MaxPowerTooHigh,
    #[error("max power is too low, check that a + b < n")]
    MaxPowerTooLow,
    #[error("histogram contains no observations")]
    Empty,
    #[error("invalid percentile, must be in range 0.0..=100.0")]
    InvalidPercentile,
    #[error("the value is outside of the storable range")]
    OutOfRange,
    #[error("the histogram parameters are incompatible")]
    IncompatibleParameters,
    #[error("the snapshot time ranges do not allow this operation")]
    IncompatibleTimeRange,
    #[error("an overflow occurred")]
    Overflow,
    #[error("unreachable code encountered")]
    Unreachable,
}
