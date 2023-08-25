use super::*;
use crate::atomic::Histogram as AtomicHistogram;
use core::sync::atomic::AtomicU64;

/// A type of histogram that reports on the distribution of values across a
/// moving window of time. For example, the distribution of values for the past
/// minute. Internally, this uses atomic histogram implementations. This allows
/// concurrent modification using atomic operations.
pub struct Histogram {
    common: Common,
    // updating: AtomicBool,

    // when the next tick begins
    tick_at: AtomicInstant,

    // the historical histogram snapshots
    snapshots: Box<[AtomicHistogram]>,

    // the live histogram, this is free-running
    live: AtomicHistogram,
}

impl _SlidingWindow for Histogram {
    fn common(&self) -> &Common {
        &self.common
    }

    fn tick_at(&self) -> Instant {
        self.tick_at.load(Ordering::Relaxed)
    }
}

impl Histogram {
    /// Create a new histogram that stores values across a sliding window and
    /// allows concurrent modification.
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
        let common = Common::new(a, b, n, interval, slices)?;

        Self::from_common(common)
    }

    /// Construct a `Histogram` from the common struct for sliding window
    /// histograms.
    pub(crate) fn from_common(common: Common) -> Result<Self, BuildError> {
        let (a, b, n) = common.params();

        let live = AtomicHistogram::new(a, b, n)?;

        let mut snapshots = Vec::with_capacity(common.num_slices());
        snapshots.resize_with(common.num_slices(), || {
            AtomicHistogram::new(a, b, n).unwrap()
        });

        let tick_at = common.tick_origin() + common.span();

        Ok(Self {
            tick_at: tick_at.into(),
            common,
            live,
            snapshots: snapshots.into(),
        })
    }

    /// Get access to the raw buckets in the live histogram.
    pub fn as_slice(&self) -> &[AtomicU64] {
        self.live.as_slice()
    }

    /// Increment the bucket that contains the value by one.
    ///
    /// This is a convenience method that uses `Instant::now()` as the time
    /// associated with the observation. If you already have a timestamp, you
    /// may wish to use `increment_at` instead.
    pub fn increment(&self, value: u64) -> Result<(), Error> {
        self.add(value, 1)
    }

    /// Increment the bucket that contains the value by some count.
    ///
    /// This is a convenience method that uses `Instant::now()` as the time
    /// associated with the observation. If you already have a timestamp, you
    /// may wish to use the `add_at` instead.
    pub fn add(&self, value: u64, count: u64) -> Result<(), Error> {
        self.add_at(Instant::now(), value, count)
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
    pub fn increment_at(&self, instant: Instant, value: u64) -> Result<(), Error> {
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
    pub fn add_at(&self, instant: Instant, value: u64, count: u64) -> Result<(), Error> {
        self.tick_to(instant);

        self.live.add(value, count)
    }

    /// Calculate and return the histogram from a previous instant to the
    /// current moment.
    ///
    /// An error will be returned if the previous instant is outside of the
    /// sliding window.
    pub fn distribution_since(&self, previous: Instant) -> Result<crate::Histogram, Error> {
        self.tick_to(Instant::now());

        let previous = &self.snapshots[self.index(previous)?].buckets;

        let (a, b, n) = self.live.config().params();
        let mut histogram = crate::Histogram::new(a, b, n).unwrap();

        for (idx, value) in previous
            .iter()
            .zip(self.live.buckets.iter())
            .map(|(previous, live)| {
                live.load(Ordering::Relaxed)
                    .wrapping_sub(previous.load(Ordering::Relaxed))
            })
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
    pub fn distribution_last(&self, duration: Duration) -> Result<crate::Histogram, Error> {
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
        &self,
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
        &self,
        duration: Duration,
        percentiles: &[f64],
    ) -> Result<Vec<(f64, Bucket)>, Error> {
        let h = self.distribution_last(duration)?;
        h.percentiles(percentiles)
    }

    /// Moves the window forward, if necessary.
    fn tick_to(&self, instant: Instant) {
        loop {
            let tick_at = self.tick_at.load(Ordering::Relaxed);

            // fast path when the window does not need to be advanced
            if instant < tick_at {
                return;
            }

            // otherwise we need to slide the window forward

            // To actually snapshot, let's just move the tick_at forward to
            // unblock other increments. This will slightly smear things into
            // the snapshot that occur after the end boundary, but this
            // trade-off seems worth it to reduce pause duration.

            let tick_next = tick_at + self.common.interval();

            // cas and if we lose, loop back, another thread may have won
            if self
                .tick_at
                .compare_exchange(tick_at, tick_next, Ordering::AcqRel, Ordering::Relaxed)
                .is_err()
            {
                continue;
            }

            // we won the race, let's snapshot

            // get the index to snapshot into
            let index = self.index(tick_at).unwrap();

            // we copy from the live slice into the start slice (since it's the oldest)
            let src = self.live.as_slice();
            let dst = self.snapshots[index].as_slice();

            for (s, d) in src.iter().zip(dst) {
                d.store(s.load(Ordering::Relaxed), Ordering::Relaxed);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn size() {
        assert_eq!(std::mem::size_of::<Histogram>(), 128);
    }

    #[test]
    fn smoke() {
        let h = Histogram::new(0, 7, 64, core::time::Duration::from_millis(1), 11)
            .expect("couldn't make histogram");
        let d = h
            .distribution_last(Duration::from_nanos(10_000_000))
            .expect("failed to get distribution");
        assert!(d.percentile(100.0).is_err());

        let _ = h.increment(100);
        let d = h
            .distribution_last(Duration::from_nanos(10_000_000))
            .expect("failed to get distribution");
        assert_eq!(d.percentile(100.0).map(|b| b.upper()), Ok(100));

        // long sleep, but ensures we don't have weird timing issues in CI
        std::thread::sleep(core::time::Duration::from_millis(20));
        let d = h
            .distribution_last(Duration::from_nanos(10_000_000))
            .expect("failed to get distribution");
        assert!(
            d.percentile(100.0).is_err(),
            "percentile is: {}",
            d.percentile(100.0).unwrap().upper()
        );
    }
}
