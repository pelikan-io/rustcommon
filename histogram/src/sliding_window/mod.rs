//! Sliding window histograms which retain samples within a sliding window of
//! time.
//!
//! These types are useful when you want to report on the behavior over some
//! recent time window. For instance, reporting on latency percentiles for the
//! past minute in a running service.
//!
//! Internally, these types are implemented as ring buffers of free-running
//! histograms. Each position in the ring buffer holds a snapshot of the live
//! free-running histogram. Time moves forward on both read and write operations
//! in snapshot length intervals. The sliding window includes some number of
//! slices each with the same duration.
//!
//! Older increments age-out as their positions in the ring buffer are replaced
//! with new snapshots.

use crate::*;

pub mod atomic;

mod standard;

pub use standard::Histogram;

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

/// Private trait for sliding window histograms to share indexing functionality.
pub(crate) trait _SlidingWindow {
    /// Get a reference to the common structure for sliding window histograms.
    fn common(&self) -> &Common;

    /// Get the snapshot index for a given instant.
    fn index(&self, instant: Instant) -> Result<usize, Error> {
        let tick_at = self.tick_at();

        let window_start = tick_at - self.common().span();
        let window_end = tick_at;

        if instant < window_start {
            return Err(Error::OutOfSlidingWindow);
        }

        if instant > window_end {
            return Err(Error::OutOfSlidingWindow);
        }

        let offset = ((instant - self.common().tick_origin()).as_nanos()
            / self.common().interval().as_nanos()) as usize
            % self.common().num_slices();

        Ok(offset)
    }

    /// Get the time when the window will move forward next
    fn tick_at(&self) -> Instant;
}

/// Common components of a sliding window histogram.
#[derive(Clone, Copy)]
pub struct Common {
    interval: Duration,
    span: Duration,
    started: UnixInstant,
    tick_origin: Instant,
    params: (u8, u8, u8),
}

impl Common {
    /// Create a new struct containing common components of a sliding window
    /// histogram.
    pub fn new(
        a: u8,
        b: u8,
        n: u8,
        interval: core::time::Duration,
        slices: usize,
    ) -> Result<Self, BuildError> {
        let started = UnixInstant::now();
        let now = Instant::now();

        let interval: u128 = interval.as_nanos();

        assert!(interval <= u64::MAX.into());
        assert!(interval >= 1000);

        let span = Duration::from_nanos(interval as u64 * slices as u64);
        let interval = Duration::from_nanos(interval as u64);

        // used to validate the other parameters
        let _ = config::Config::new(a, b, n)?;

        let tick_origin = now - span;

        Ok(Self {
            interval,
            span,
            started,
            tick_origin,
            params: (a, b, n),
        })
    }

    /// The number of snapshots to represent the sliding window.
    pub fn num_slices(&self) -> usize {
        1 + (self.span.as_nanos() / self.interval.as_nanos()) as usize
    }

    /// The instant the histogram began
    pub fn tick_origin(&self) -> Instant {
        self.tick_origin
    }

    /// The interval between snapshots
    pub fn interval(&self) -> Duration {
        self.interval
    }

    /// The total duration for the sliding window
    pub fn span(&self) -> Duration {
        self.span
    }

    /// The `a`, `b`, and `n` parameters for the histograms
    pub fn params(&self) -> (u8, u8, u8) {
        self.params
    }
}

#[cfg(test)]
mod test {
    use super::atomic::Histogram;
    use super::Common;
    use crate::sliding_window::_SlidingWindow;
    use crate::Error;
    use clocksource::precise::Duration;

    #[test]
    fn size() {
        assert_eq!(std::mem::size_of::<Common>(), 40);
    }

    #[test]
    fn indexing() {
        let h = Histogram::new(0, 7, 64, core::time::Duration::from_secs(1), 60).unwrap();
        let now = h.common().tick_origin();

        assert_eq!(h.index(now), Ok(0));
        assert_eq!(h.index(now + Duration::from_secs(1)), Ok(1));
        assert_eq!(h.index(now + Duration::from_secs(59)), Ok(59));
        assert_eq!(h.index(now + Duration::from_secs(60)), Ok(60));

        assert_eq!(
            h.index(now - Duration::from_secs(1)),
            Err(Error::OutOfSlidingWindow)
        );
        assert_eq!(
            h.index(now + Duration::from_secs(61)),
            Err(Error::OutOfSlidingWindow)
        );

        assert_eq!(h.index(h.tick_at()), Ok(60));
    }
}
