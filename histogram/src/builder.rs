use crate::Instant;
use crate::UnixInstant;
use crate::BuildError;

/// A builder that can be used to construct a sliding window histogram.
///
/// By using the `Builder` you can specify a start instant for the histogram.
pub struct Builder {
    common: Common,
}

impl Builder {
    /// Create a new builder for constructing a sliding window histogram.
    ///
    /// # Parameters:
    /// * `a` sets bin width in the linear portion, the bin width is `2^a`
    /// * `b` sets the number of divisions in the logarithmic portion to `2^b`.
    /// * `n` sets the max value as `2^n`. Note: when `n` is 64, the max value
    ///   is `u64::MAX`
    /// * `interval` is the duration of each discrete time slice
    /// * `slices` is the number of discrete time slices
    ///
    /// # Constraints:
    /// * `n` must be less than or equal to 64
    /// * `n` must be greater than `a + b`
    /// * `interval` in nanoseconds must fit within a `u64`
    /// * `interval` must be at least 1 microsecond
    pub fn new(
        a: u8,
        b: u8,
        n: u8,
        interval: core::time::Duration,
        slices: usize,
    ) -> Result<Self, BuildError> {
        Ok(Self {
            common: Common::new(a, b, n, interval, slices)?,
        })
    }

    /// Specify the start time for the histogram as a `UnixInstant`.
    pub fn start_unix(mut self, start: UnixInstant) -> Self {
        if self.common.started < start {
            let delta = start - self.common.started;
            self.common.started += delta;
            self.common.tick_origin += delta;
        } else {
            let delta = self.common.started - start;
            self.common.started -= delta;
            self.common.tick_origin -= delta;
        }
        self
    }

    /// Specify the start time for the histogram as an `Instant`.
    pub fn start_instant(mut self, start: Instant) -> Self {
        if self.common.tick_origin < start {
            let delta = start - self.common.tick_origin;
            self.common.started += delta;
            self.common.tick_origin += delta;
        } else {
            let delta = self.common.tick_origin - start;
            self.common.started -= delta;
            self.common.tick_origin -= delta;
        }
        self
    }

    /// Consume the builder and produce a sliding window histogram that uses
    /// atomic operations.
    pub fn atomic(self) -> Result<atomic::Histogram, BuildError> {
        atomic::Histogram::from_common(self.common)
    }

    /// Consume the builder and produce a sliding window histogram.
    pub fn standard(self) -> Result<Histogram, BuildError> {
        Histogram::from_common(self.common)
    }
}