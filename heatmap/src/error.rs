// Copyright 2020 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use histogram::Error as HistogramError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum Error {
    #[error("heatmap contains no samples")]
    /// The heatmap contains no samples.
    Empty,
    #[error("invalid percentile")]
    /// The provided percentile is outside of the range 0.0 - 100.0 (inclusive)
    InvalidPercentile,
    #[error("value out of bucket range")]
    /// The provided value is outside of the representable range.
    OutOfRange,
    #[error("value out of heatmap span")]
    /// The provided timestamp indicates a time no longer tracked by the `Heatmap`.
    OutOfSpan,
    #[error("heatmap clock became stale")]
    /// The `Heatmap`'s internal clock reading fell behind by more than one tick.
    StaleClock,
    #[error("invalid heatmap config")]
    /// The heatmap configuration is invalid, see docs for `Heatmap::new()` for
    /// the constraints.
    InvalidConfig,
}

impl From<HistogramError> for Error {
    fn from(other: HistogramError) -> Self {
        match other {
            HistogramError::Empty => Self::Empty,
            HistogramError::InvalidPercentile => Self::InvalidPercentile,
            HistogramError::OutOfRange => Self::OutOfRange,
            HistogramError::InvalidConfig => Self::InvalidConfig,
            HistogramError::IncompatibleHistogram => {
                // SAFETY: a heatmap has histograms which all have the same
                // configuration and therefore the operations which act on two
                // histograms will always have two compatible histograms
                panic!("imposible state")
            }
        }
    }
}
