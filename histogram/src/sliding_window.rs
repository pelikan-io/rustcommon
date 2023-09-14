//! A histogram that stores a distribution for a fixed window of time.

use crate::{
    AtomicInstant, BuildError, Config, Duration, Error, Instant, Ordering, Range, Snapshot,
    UnixInstant,
};
use core::sync::atomic::AtomicU64;

/// A type of histogram that reports on the distribution of values across a
/// moving window of time. For example, the distribution of values for the past
/// minute. Internally, this uses atomic counters to allow concurrent
/// modification.
pub struct Histogram {
    config: Config,
    interval: Duration,
    span: Duration,
    started: UnixInstant,
    tick_origin: Instant,
    tick_at: AtomicInstant,
    num_slices: usize,
    snapshots: Box<[Box<[AtomicU64]>]>,
    live: Box<[AtomicU64]>,
}

/// A builder that can be used to construct a sliding window histogram.
///
/// By using the `Builder` you can specify a start instant for the histogram.
pub struct Builder {
    config: Config,
    interval: core::time::Duration,
    slices: usize,
    started: Option<UnixInstant>,
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
        let config = Config::new(a, b, n)?;

        Ok(Self {
            config,
            interval,
            slices,
            started: None,
        })
    }

    /// Specify the start time for the histogram as a `UnixInstant`.
    pub fn start(mut self, start: UnixInstant) -> Self {
        self.started = Some(start);
        self
    }

    /// Consume the builder and produce a sliding window histogram that uses
    /// atomic operations.
    pub fn build(self) -> Result<Histogram, BuildError> {
        let (a, b, n) = self.config.params();

        let mut h = Histogram::new(a, b, n, self.interval, self.slices)?;

        // if we have some start time, we move the three time fields in the
        // histogram as necessary
        if let Some(start) = self.started {
            if start < h.started {
                let delta = h.started - start;
                h.started -= delta;
                h.tick_origin -= delta;
                h.tick_at.fetch_sub(delta, Ordering::Relaxed);
            } else {
                let delta = start - h.started;
                h.started += delta;
                h.tick_origin += delta;
                h.tick_at.fetch_add(delta, Ordering::Relaxed);
            }
        }

        Ok(h)
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
    /// * `interval` must be at least 1 millisecond
    pub fn new(
        a: u8,
        b: u8,
        n: u8,
        interval: core::time::Duration,
        slices: usize,
    ) -> Result<Self, BuildError> {
        let now = Instant::now();
        let started = UnixInstant::now();

        let config = Config::new(a, b, n)?;

        let mut live = Vec::with_capacity(config.total_bins());
        live.resize_with(config.total_bins(), || AtomicU64::new(0));

        let interval: u128 = interval.as_nanos();

        if interval >= Duration::SECOND.as_nanos() as u128 * 3600 {
            return Err(BuildError::IntervalTooLong);
        }

        if interval < Duration::MILLISECOND.as_nanos() as u128 {
            return Err(BuildError::IntervalTooShort);
        }

        let span = Duration::from_nanos(interval as u64 * slices as u64);
        let interval = Duration::from_nanos(interval as u64);

        let started = started - span;
        let tick_origin = now - span;
        let tick_at = now;

        let num_slices = 1 + (span.as_nanos() / interval.as_nanos()) as usize;

        let mut snapshots = Vec::with_capacity(num_slices);
        snapshots.resize_with(num_slices, || {
            let mut snapshot = Vec::with_capacity(config.total_bins());
            snapshot.resize_with(config.total_bins(), || AtomicU64::new(0));
            snapshot.into()
        });

        Ok(Self {
            config,
            interval,
            span,
            started,
            tick_origin,
            tick_at: tick_at.into(),
            num_slices,
            live: live.into(),
            snapshots: snapshots.into(),
        })
    }

    /// Get access to the raw buckets in the live histogram.
    ///
    /// This is useful if you need access to the raw bucket counts or if you are
    /// planning to update from some external source that uses the same
    /// bucketing strategy.
    pub fn as_slice(&self) -> &[AtomicU64] {
        self.snapshot();
        &self.live
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
    /// the most recent time slice regardless of the true position within the
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
    /// the most recent time slice regardless of the true position within the
    /// sliding window.
    pub fn add_at(&self, instant: Instant, value: u64, count: u64) -> Result<(), Error> {
        self.tick_to(instant);

        let index = self.config.value_to_index(value)?;

        self.live[index].fetch_add(count, Ordering::Relaxed);

        Ok(())
    }

    /// Returns a snapshot that covers the provided range. Both the start and
    /// end of the range will be adjusted to the proceeding snapshot (tick)
    /// boundary. This results in distribution between the start and end times,
    /// which includes the provided start but excludes the provided end.
    pub fn snapshot_between(
        &self,
        range: core::ops::Range<UnixInstant>,
    ) -> Result<crate::Snapshot, Error> {
        self.snapshot();

        let tick_at = self.tick_at();

        if range.start < self.started {
            return Err(Error::OutOfSlidingWindow);
        }

        // convert unix times to monotonic clock times
        let start = self.tick_origin + (range.start - self.started - self.interval);
        let end = self.tick_origin + (range.end - self.started - self.interval);

        // lookup snapshot information
        let start = self.snapshot_info(start, tick_at)?;
        let end = self.snapshot_info(end, tick_at)?;

        let mut total_count = 0_u128;

        let buckets: Vec<u64> = self.snapshots[start.index]
            .iter()
            .zip(self.snapshots[end.index].iter())
            .map(|(start, end)| {
                let count = end
                    .load(Ordering::Relaxed)
                    .wrapping_sub(start.load(Ordering::Relaxed));
                total_count += count as u128;
                count
            })
            .collect();

        let histogram = crate::Histogram {
            config: self.config,
            total_count,
            buckets: buckets.into(),
        };

        Ok(Snapshot {
            range: start.range.start..end.range.end,
            histogram,
        })
    }

    /// Returns the current inclusive range of time covered by the histogram.
    pub fn range(&self) -> Range<UnixInstant> {
        let elapsed = self.tick_at.load(Ordering::Relaxed) - self.interval - self.tick_origin;
        let end = self.started + elapsed;
        let start = end - self.span;

        start..end
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

            let tick_next = tick_at + self.interval;

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
            let index = self.snapshot_info(tick_at, tick_next).unwrap().index;

            // we copy from the live slice into the start slice (since it's the oldest)
            let src = &self.live;
            let dst = &self.snapshots[index];

            for (s, d) in src.iter().zip(dst.iter()) {
                d.store(s.load(Ordering::Relaxed), Ordering::Relaxed);
            }
        }
    }

    /// Get the time when the data structure will tick forward next.
    fn tick_at(&self) -> Instant {
        self.tick_at.load(Ordering::Relaxed)
    }

    // Get the snapshot info for a given instant relative to when the data
    // structure will tick forward next.
    fn snapshot_info(&self, instant: Instant, tick_at: Instant) -> Result<SnapshotInfo, Error> {
        if instant < self.tick_origin {
            return Err(Error::OutOfSlidingWindow);
        }

        let window_end = tick_at - self.interval;
        let window_start = window_end - self.span;

        if instant < window_start {
            return Err(Error::OutOfSlidingWindow);
        }

        if instant > window_end {
            return Err(Error::OutOfSlidingWindow);
        }

        let ticks = (instant - self.tick_origin).as_nanos() / self.interval.as_nanos();

        let index = ticks as usize % self.num_slices;

        let offset_ns = Duration::from_nanos(ticks * self.interval.as_nanos());

        let start = self.started + offset_ns;
        let end = start + self.interval;

        let range = core::ops::Range { start, end };

        Ok(SnapshotInfo { index, range })
    }

    /// Causes the histogram window to slide forward to the current time, if
    /// necessary.
    ///
    /// This is useful if you are updating the live buckets directly.
    fn snapshot(&self) {
        self.tick_to(Instant::now());
    }
}

#[derive(Debug, PartialEq)]
struct SnapshotInfo {
    index: usize,
    range: Range<UnixInstant>,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn size() {
        assert_eq!(std::mem::size_of::<Histogram>(), 112);
    }

    #[test]
    fn indexing() {
        let h = Histogram::new(0, 7, 64, core::time::Duration::from_secs(1), 60).unwrap();
        let now = h.tick_origin;
        let tick_at = h.tick_at();

        assert_eq!(h.snapshot_info(now, tick_at).map(|v| v.index), Ok(0));
        assert_eq!(
            h.snapshot_info(now + Duration::from_secs(1), tick_at)
                .map(|v| v.index),
            Ok(1)
        );
        assert_eq!(
            h.snapshot_info(now + Duration::from_secs(59), tick_at)
                .map(|v| v.index),
            Ok(59)
        );
        assert_eq!(
            h.snapshot_info(now + Duration::from_secs(60), tick_at),
            Err(Error::OutOfSlidingWindow)
        );

        assert_eq!(
            h.snapshot_info(now - Duration::from_secs(1), tick_at),
            Err(Error::OutOfSlidingWindow)
        );
        assert_eq!(
            h.snapshot_info(now + Duration::from_secs(61), tick_at),
            Err(Error::OutOfSlidingWindow)
        );

        assert_eq!(
            h.snapshot_info(h.tick_at(), tick_at),
            Err(Error::OutOfSlidingWindow)
        );
    }

    #[test]
    fn smoke() {
        // histogram is initially empty
        let h = Histogram::new(0, 7, 64, core::time::Duration::from_millis(1), 11)
            .expect("couldn't make histogram");
        let end = UnixInstant::now();
        let s = h
            .snapshot_between((end - Duration::from_millis(10))..end)
            .expect("failed to get distribution");
        assert!(s.percentile(100.0).is_err());

        // after incrementing and with one or more intervals elapsed, the
        let _ = h.increment(100);
        std::thread::sleep(core::time::Duration::from_millis(2));
        let end = UnixInstant::now();
        let s = h
            .snapshot_between((end - Duration::from_millis(10))..end)
            .expect("failed to get distribution");
        assert_eq!(s.percentile(100.0).map(|b| b.end()), Ok(100));

        // long sleep, but ensures we don't have weird timing issues in CI
        std::thread::sleep(core::time::Duration::from_millis(20));
        let end = UnixInstant::now();
        let s = h
            .snapshot_between((end - Duration::from_millis(10))..end)
            .expect("failed to get distribution");
        assert!(
            s.percentile(100.0).is_err(),
            "percentile is: {}",
            s.percentile(100.0).unwrap().end()
        );
    }
}
