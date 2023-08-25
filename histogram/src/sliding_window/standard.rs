use super::*;

/// A type of histogram that reports on the distribution of values across a
/// moving window of time. For example, the distribution of values for the past
/// minute.
pub struct Histogram {
    common: Common,

    // when the next tick begins
    tick_at: Instant,

    // the historical histogram snapshots
    snapshots: Box<[crate::Histogram]>,

    // the live histogram, this is free-running
    live: crate::Histogram,
}

impl _SlidingWindow for Histogram {
    fn common(&self) -> &Common {
        &self.common
    }

    fn tick_at(&self) -> Instant {
        self.tick_at
    }
}

impl Histogram {
    /// Create a new `SlidingWindowHistogram` given the provided parameters.
    ///
    /// Construct a new `SlidingWindowHistogram` from the provided parameters.
    /// * `a` sets bin width in the linear portion, the bin width is `2^a`
    /// * `b` sets the number of divisions in the logarithmic portion to `2^b`.
    /// * `n` sets the max value as `2^n`. Note: when `n` is 64, the max value
    ///   is `u64::MAX`
    /// * `interval` is the duration of each discrete time slice
    /// * `slices` is the number of discrete time slices
    ///
    /// # Constraints
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
        let common = Common::new(a, b, n, interval, slices)?;

        Self::from_common(common)
    }

    /// Construct a `Histogram` from the common struct for sliding window
    /// histograms.
    pub(crate) fn from_common(common: Common) -> Result<Self, BuildError> {
        let (a, b, n) = common.params();

        let live = crate::Histogram::new(a, b, n)?;

        let mut snapshots = Vec::with_capacity(common.num_slices());
        snapshots.resize_with(common.num_slices(), || {
            crate::Histogram::new(a, b, n).unwrap()
        });

        Ok(Self {
            tick_at: common.tick_origin() + common.interval(),
            common,
            live,
            snapshots: snapshots.into(),
        })
    }

    /// Increment the bucket that contains the value by some count.
    ///
    /// This is a convenience method that uses `Instant::now()` as the time
    /// associated with the observation. If you already have a timestamp, you
    /// may wish to use the `add_at` instead.
    pub fn add(&mut self, value: u64, count: u64) -> Result<(), Error> {
        self.add_at(Instant::now(), value, count)
    }

    /// Increment the bucket that contains the value by one.
    ///
    /// This is a convenience method that uses `Instant::now()` as the time
    /// associated with the observation. If you already have a timestamp, you
    /// may wish to use `increment_at` instead.
    pub fn increment(&mut self, value: u64) -> Result<(), Error> {
        self.add(value, 1)
    }

    /// Increment time-value pair by one.
    ///
    /// If the instant is after the current sliding window, the window will
    /// slide forward so that the window included the instant before the
    /// increment is recorded.
    ///
    /// If the instant is earlier than the start of the sliding window, an error
    /// will be returned.
    ///
    /// If the instant is within the window, the increment will be attributed to
    /// the most recent time slide regardless of the true position within the
    /// sliding window.
    pub fn increment_at(&mut self, instant: Instant, value: u64) -> Result<(), Error> {
        self.add_at(instant, value, 1)
    }

    /// Increment a time-value pair by some count.
    ///
    /// If the instant is after the current sliding window, the window will
    /// slide forward so that the window included the instant before the
    /// increment is recorded.
    ///
    /// If the instant is earlier than the start of the sliding window, an error
    /// will be returned.
    ///
    /// If the instant is within the window, the increment will be attributed to
    /// the most recent time slide regardless of the true position within the
    /// sliding window.
    pub fn add_at(&mut self, instant: Instant, value: u64, count: u64) -> Result<(), Error> {
        self.tick_to(instant);

        // and finally record into the live histogram
        self.live.add(value, count)
    }

    /// Calculate and return the histogram from a previous instant to the
    /// current moment.
    ///
    /// An error will be returned if the previous instant is outside of the
    /// sliding window.
    pub fn distribution_since(&mut self, previous: Instant) -> Result<crate::Histogram, Error> {
        self.tick_to(Instant::now());

        let previous = &self.snapshots[self.index(previous)?].buckets;

        let (a, b, n) = self.live.config().params();
        let mut histogram = crate::Histogram::new(a, b, n).unwrap();

        for (idx, value) in previous
            .iter()
            .zip(self.live.buckets.iter())
            .map(|(previous, live)| live.wrapping_sub(*previous))
            .enumerate()
        {
            histogram.buckets[idx] = value;
        }

        Ok(histogram)
    }

    /// Calculate and return the histogram that covers a fixed duration. This is
    /// a convenience wrapper around `distribution_since`.
    ///
    /// An error will be returned if the duration is longer than the sliding
    /// window.
    pub fn distribution_last(&mut self, duration: Duration) -> Result<crate::Histogram, Error> {
        let now = Instant::now();
        let previous = now - duration;
        self.distribution_since(previous)
    }

    /// Calculate and return the specified percentiles from a previous instant
    /// to the current moment.
    ///
    /// An error will be returned if the previous instant is outside of the
    /// sliding window.
    pub fn percentiles_since(
        &mut self,
        previous: Instant,
        percentiles: &[f64],
    ) -> Result<Vec<(f64, Bucket)>, Error> {
        let h = self.distribution_since(previous)?;
        h.percentiles(percentiles)
    }

    /// Calculate and return the specified percentiles from a previous instant
    /// to the current moment. This is a convenience wrapper around
    /// `percentiles_since`.
    ///
    /// An error will be returned if the duration is longer than the sliding
    /// window.
    pub fn percentiles_last(
        &mut self,
        duration: Duration,
        percentiles: &[f64],
    ) -> Result<Vec<(f64, Bucket)>, Error> {
        let h = self.distribution_last(duration)?;
        h.percentiles(percentiles)
    }

    /// Moves the window forward, if necessary.
    fn tick_to(&mut self, instant: Instant) {
        let tick_at = self.tick_at;

        // fast path, we just update the live histogram
        if instant < tick_at {
            // if instant < (tick_at - self.resolution) {
            // We *could* attempt to record into prior snapshots. But
            // for simplicity and to avoid changing past readings, we
            // will simply record into the live histogram anyway. We
            // might want to raise this as an error.
            // }

            return;
        }

        // rarer path where we need to snapshot
        //
        // Even if we are behind by multiple ticks, we will only snapshot
        // into the most recent snapshot position. This ensures that we will
        // not change past readings. It also simplifies things and reduces
        // the number of load/store operations.

        let tick_next = self.tick_at + self.common.interval();

        self.tick_at = tick_next;

        // calculate the indices for the previous start and end snapshots
        let index = self.index(tick_at).unwrap();

        // we copy from the live slice into the start slice (since it's the oldest)
        let src = self.live.as_slice();
        let dst = self.snapshots[index].as_mut_slice();

        dst.copy_from_slice(src);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn size() {
        assert_eq!(std::mem::size_of::<Histogram>(), 128);
    }
}
