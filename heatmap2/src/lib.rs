mod atomic_histogram;
mod histogram;

pub use histogram::{Bucket, Histogram};
pub use atomic_histogram::AtomicHistogram;

use parking_lot::Mutex;
use core::sync::atomic::{Ordering};
use clocksource::datetime::DateTime;
use clocksource::precise::{AtomicInstant, Duration, Instant, UnixInstant};

struct Snapshots {
    write_ptr: usize,
    len: usize,
    mask: usize,
    scratch: Histogram,
    histograms: Box<[(DateTime, Histogram)]>,
}

impl Snapshots {
    pub fn new(a: u8, b: u8, n: u8, count: usize) -> Self {
        assert!(count > 0);

        let now = DateTime::from(UnixInstant::now());

        let mut histograms = Vec::with_capacity(count);
        histograms.resize_with(count, || { (now, Histogram::new(a, b, n)) });

        Self {
            write_ptr: 0,
            len: 0,
            mask: count - 1,
            scratch: Histogram::new(a, b, n),
            histograms: histograms.into(),
        }
    }

    pub fn push(&mut self, histogram: &AtomicHistogram) {
        assert_eq!(histogram.buckets.len(), self.histograms[0].1.buckets.len());

        let write_idx = self.write_ptr & self.mask;

        self.histograms[write_idx].0 = DateTime::from(UnixInstant::now());

        for (idx, count) in histogram.buckets.iter().enumerate() {
            self.histograms[write_idx].1.buckets[idx] = count.load(Ordering::Relaxed);
        }

        self.write_ptr += 1;

        if self.len < self.histograms.len() - 1 {
            self.len += 1;
        }
    }

    pub fn percentiles(&mut self, lookback: usize, percentiles: &[f64]) -> Option<Vec<(f64, Bucket)>> {
        if lookback > self.len {
            return None;
        }

        let write_idx = self.write_ptr & self.mask;

        let newest = if write_idx == 0 {
            self.histograms.len() - 1
        } else {
            write_idx - 1
        };

        let oldest = if newest >= lookback {
            newest - lookback
        } else {
            newest + self.histograms.len() - lookback
        };

        for (idx, v) in self.histograms[newest].1.buckets.iter().enumerate() {
            self.scratch.buckets[idx] = *v;
        }
        for (idx, v) in self.histograms[oldest].1.buckets.iter().enumerate() {
            self.scratch.buckets[idx] = self.scratch.buckets[idx].wrapping_sub(*v);
        }

        self.scratch.percentiles(percentiles)
    }
}

pub struct MovingWindowHistogram {
    live: AtomicHistogram,
    tick_start: AtomicInstant,
    tick_stop: AtomicInstant,
    resolution: Duration,
    snapshots: Mutex<Snapshots>,
}

impl MovingWindowHistogram {
    pub fn new(a: u8, b: u8, n: u8, resolution: core::time::Duration, slices: usize) -> Self {

        let now = Instant::now();

        let resolution: u128 = resolution.as_nanos();

        assert!(resolution <= u64::MAX.into());
        assert!(resolution > 0);

        let resolution = Duration::from_nanos(resolution as u64);

        Self {
            live: AtomicHistogram::new(a, b, n),
            tick_start: now.into(),
            tick_stop: (now + resolution).into(),
            resolution,
            snapshots: Snapshots::new(a, b, n, slices).into(),
        }
    }

    /// Increment the bucket that contains the value by one. This is a
    /// convenience method that uses `Timestamp::now()` as the time associated
    /// withe the observation. If you already have a timestamp, please use
    /// `increment_at` instead.
    pub fn increment(&self, value: u64) {
        self.increment_at(Instant::now(), value)
    }

    /// Increment a timestamp-value pair by one. This is useful if you
    /// already have done the timestamping elsewhere. For example, if tracking
    /// latency measurements, you have the timestamps for the start and end of
    /// the event and it would be wasteful to timestamp again.
    pub fn increment_at(&self, instant: Instant, value: u64) {
        loop {
            if instant < self.tick_stop.load(Ordering::Relaxed) {
                if instant < self.tick_start.load(Ordering::Relaxed) {
                    // this was too early, record into the current time slice
                    // but we should also log the event
                }

                self.live.increment(value);
                return;
            }

            // attempt to lock the snapshots for update
            //
            // note: other increments will block while we're updating
            if let Some(mut snapshots) = self.snapshots.try_lock() {
                // we successfully moved forward by one, we need to push a
                // snapshot of the live histogram
                snapshots.push(&self.live);

                self.tick_stop.fetch_add(self.resolution, Ordering::Relaxed);
                self.tick_start.fetch_add(self.resolution, Ordering::Relaxed);
            }

            // if we didn't lock, repeat loop to check the current `tick_at`
        }
    }

    pub fn percentiles(&self, duration: core::time::Duration, percentiles: &[f64]) -> Option<Vec<(f64, Bucket)>> {
        let lookback = duration.as_nanos() as u64 / self.resolution.as_nanos();

        let mut snapshots = self.snapshots.lock();

        snapshots.percentiles(lookback as usize, percentiles)
    }
}