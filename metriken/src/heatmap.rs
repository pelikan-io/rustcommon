use crate::{Metric, Value};

use std::sync::OnceLock;
use std::time::Duration;

use heatmap::Instant;

pub use ::heatmap::Bucket;
pub use ::heatmap::Error as HeatmapError;
pub use ::heatmap::Iter as HeatmapIter;
pub use ::heatmap::Percentile;

/// A heatmap holds counts for quantized values across a period of time. It can
/// be used to record observations at points in time and report out percentile
/// metrics or the underlying distribution.
///
/// Common use cases of heatmaps include any per-event measurement such as
/// latency or size. Alternate use cases include summarizing fine-grained
/// observations (sub-secondly rates, or sub-secondly gauge readings) into
/// percentiles across a period of time.
///
/// Heatmaps are lazily initialized, which means that read methods will return
/// a None variant until some write has occured. This also means they occupy
/// very little space until they are initialized.
pub struct Heatmap {
    inner: OnceLock<heatmap::Heatmap>,
    m: u32,
    r: u32,
    n: u32,
    span: Duration,
    resolution: Duration,
}

impl Heatmap {
    /// Create a new heatmap with the given parameters.
    ///
    /// - `m` - sets the minimum resolution `M = 2^m`. This is the smallest unit
    /// of quantification, which is also the smallest bucket width. If the input
    /// values are always integers, choosing `m=0` would ensure precise
    /// recording for the smallest values.
    ///
    /// - `r` - sets the minimum resolution range `R = 2^r - 1`. The selected
    /// value must be greater than the minimum resolution `m`. This sets the
    /// maximum value that the minimum resolution should extend to.
    ///
    /// - `n` - sets the maximum value `N = 2^n - 1`. The selected value must be
    /// greater than or equal to the minimum resolution range `r`.
    ///
    /// - `span` - sets the total window of time that the heatmap will cover.
    /// Observations that are older than the span will age out.
    ///
    /// - `resolution` - sets the resolution in the time domain. The times of
    /// observations are quantized into slices of this duration. Entire slices
    /// are aged out of the heatmap as necessary.
    pub const fn new(m: u32, r: u32, n: u32, span: Duration, resolution: Duration) -> Self {
        Self {
            m,
            r,
            n,
            span,
            resolution,
            inner: OnceLock::new(),
        }
    }

    /// Returns the `Bucket` (if any) where the requested percentile falls
    /// within the value range for the bucket. Percentiles should be expressed
    /// as a value in the range `0.0..=100.0`.
    ///
    /// `None` will be returned if the heatmap has not been written to.
    pub fn percentile(&self, percentile: f64) -> Option<Result<Bucket, HeatmapError>> {
        self.inner
            .get()
            .map(|heatmap| heatmap.percentile(percentile))
    }

    /// Retrieves multiple percentiles in one operation. This is more efficient
    /// than calling `percentile()` multiple times.
    pub fn percentiles(
        &self,
        percentiles: &[f64],
    ) -> Option<Result<Vec<Percentile>, HeatmapError>> {
        self.inner
            .get()
            .map(|heatmap| heatmap.percentiles(percentiles))
    }

    /// Increments a time-value pair by one.
    pub fn increment(&self, time: Instant, value: u64) -> Result<(), HeatmapError> {
        self.add(time, value, 1)
    }

    /// Increments a time-value pair by some count.
    pub fn add(&self, time: Instant, value: u64, count: u32) -> Result<(), HeatmapError> {
        self.get_or_init().increment(time, value, count)
    }

    pub fn iter(&self) -> Option<HeatmapIter> {
        self.inner.get().map(|heatmap| heatmap.iter())
    }

    fn get_or_init(&self) -> &::heatmap::Heatmap {
        self.inner.get_or_init(|| {
            ::heatmap::Heatmap::new(
                self.m,
                self.r,
                self.n,
                ::heatmap::Duration::from_nanos(self.span.as_nanos() as u64),
                ::heatmap::Duration::from_nanos(self.resolution.as_nanos() as u64),
                None,
                None,
            )
            .unwrap()
        })
    }
}

impl Metric for Heatmap {
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn value(&self) -> Option<Value> {
        Some(Value::Heatmap(self))
    }
}
