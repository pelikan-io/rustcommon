// Copyright 2020 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::Error;
use crate::*;
use core::sync::atomic::*;

use clocksource::DateTime;
use histogram::{Bucket, Histogram};
use parking_lot::Mutex;

/// A `Heatmap` stores counts for timestamped values over a configured span of
/// time.
///
/// Internally, it is represented as a ring buffer of histograms with one
/// additional histogram ("summary") to track all counts within the span of time.
/// Each histogram covers a time slice whose width is specified by `resolution`.
/// Old histograms age-out as time moves forward and they are subtracted from the
/// summary histogram at that point.
///
/// This acts as a moving histogram, such that requesting a percentile returns
/// a percentile from across the configured span of time.
pub struct Heatmap {
    span: Duration,
    resolution: Duration,

    summary: Histogram,
    histograms: Vec<Histogram>,

    // index in the ring buffer of the current histogram, which has the latest start time
    idx: AtomicUsize,
    // start time of the current time slice
    create_at: DateTime,
    first_tick: Instant,
    curr_tick: AtomicInstant,
    next_tick: AtomicInstant,

    lock: Mutex<()>,
}

/// A `Builder` allows for constructing a `Heatmap` with the desired
/// configuration.
pub struct Builder {
    // minimum resolution parameter `M = 2^m`
    m: u32,
    // minimum resolution range parameter `R = 2^r - 1`
    r: u32,
    // maximum value parameter `N = 2^n - 1`
    n: u32,
    // span of time represented by the heatmap
    span: Duration,
    // the resolution in the time domain
    resolution: Duration,
}

impl Builder {
    /// Consume the `Builder` and return a `Heatmap`.
    pub fn build(self) -> Result<Heatmap, Error> {
        Heatmap::new(self.m, self.r, self.n, self.span, self.resolution)
    }

    /// Sets the width of the smallest bucket in the `Heatmap`.
    ///
    /// As the `Heatmap` uses base-2 internally, the resolution will be the
    /// largest power of two that is less than or equal to the provided value.
    /// For example, if the minimum resolution is set to 10, the width of the
    /// smallest bucket will be 8.
    pub fn min_resolution(mut self, width: u64) -> Self {
        self.m = 64 - width.leading_zeros();
        self
    }

    /// Sets the maximum value that the minimum resolution extends to.
    ///
    /// This value should be greater than the minimum resolution. If the value
    /// provided is not a power of two, the smallest power of two that is larger
    /// than the provided value will be used.
    pub fn min_resolution_range(mut self, value: u64) -> Self {
        self.r = 64 - value.next_power_of_two().leading_zeros();
        self
    }

    /// Sets the maximum value that can be recorded into the `Heatmap`.
    ///
    /// If the value provided is not a power of two, the smallest power of two
    /// that is larger than the provided value will be used.
    pub fn maximum_value(mut self, value: u64) -> Self {
        self.n = 64 - value.next_power_of_two().leading_zeros();
        self
    }

    /// Sets the duration that is covered by the `Heatmap`.
    ///
    /// Values that are older than the duration will be dropped as they age-out.
    /// Due to resolution constraints, the true duration covered by the heatmap
    /// may be slightly longer than what the Builder is instructed to cover,
    /// because the true duration has to be a multiple of resolution.
    pub fn span(mut self, duration: Duration) -> Self {
        self.span = duration;
        self
    }

    /// Sets the resolution in the time domain.
    ///
    /// Increments with similar timestamps will be grouped together and age-out
    /// together.
    pub fn resolution(mut self, duration: Duration) -> Self {
        self.resolution = duration;
        self
    }
}

impl Heatmap {
    /// Create a new `Heatmap` which stores counts for timestamped values over
    /// a configured span of time.
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
    /// - `span` - sets the total duration that the heatmap covers
    ///
    /// - `resolution` - sets the resolution in the time domain. Counts from
    /// similar instants in time will be grouped together.
    pub fn new(
        m: u32,
        r: u32,
        n: u32,
        span: Duration,
        resolution: Duration,
    ) -> Result<Self, Error> {
        let mut histograms = Vec::new();
        let mut true_span = Duration::from_nanos(0);
        let mut span_stop = span;
        // allocate one extra histogram so we always have a cleared
        // one in the ring
        span_stop += resolution;
        while true_span < span_stop {
            histograms.push(Histogram::new(m, r, n).unwrap());
            true_span += resolution;
        }
        histograms.shrink_to_fit();

        let create_at = DateTime::now();
        let first_tick = Instant::now();
        let curr_tick = AtomicInstant::new(first_tick);
        let next_tick = AtomicInstant::new(first_tick + resolution);

        Ok(Self {
            span: true_span,
            resolution,
            summary: Histogram::new(m, r, n)?,
            histograms,
            idx: AtomicUsize::new(0),
            create_at,
            first_tick,
            curr_tick,
            next_tick,
            lock: Mutex::new(()),
        })
    }

    /// Creates a `Builder` with the default values `m = 0`, `r = 10`, `n = 30`,
    /// `span = 60s`, `resolution = 1s`.
    ///
    /// This would create a `Heatmap` with 61 total `Histogram`s, each with
    /// 11264 buckets which can store values from 1 to 1_073_741_823 with
    /// values 1 to 1023 being stored in buckets with a width of 1. Such a
    /// `Heatmap` would be appropriate for latencies measured in nanoseconds
    /// where the max expected latency is one second and reporting covers the
    /// past minute.
    pub fn builder() -> Builder {
        Builder {
            m: 0,
            r: 10,
            n: 30,
            span: Duration::from_secs(60),
            resolution: Duration::from_secs(1),
        }
    }

    /// Returns the true span (as `Duration`) that is tracked by the `HeatMap`
    pub fn span(&self) -> Duration {
        self.span
    }

    /// Returns the `Duration` covered by a single slice of histogram
    pub fn resolution(&self) -> Duration {
        self.resolution
    }

    /// Returns the number of slices stored in the `Heatmap`
    pub fn slices(&self) -> usize {
        self.histograms.len()
    }

    /// Returns the number of buckets stored within each `Histogram` in the
    /// `Heatmap`
    pub fn buckets(&self) -> usize {
        self.summary.buckets()
    }

    /// Returns the number of valid and active `Histogram` slices in the `Heatmap`
    pub fn active_slices(&self) -> usize {
        if Instant::now() - self.first_tick > self.span {
            self.slices()
        } else {
            self.idx.load(Ordering::Relaxed) + 1
        }
    }

    /// Returns the DateTime representation of when the `HeatMapi` was created
    pub fn created_at(&self) -> DateTime {
        self.create_at
    }

    /// Increment a time-value pair by a specified count
    pub fn increment(&self, time: Instant, value: u64, count: u32) {
        self.tick(time);

        let curr_tick = self.curr_tick.load(Ordering::Relaxed);
        let idx = self.idx.load(Ordering::Relaxed);

        // fast path when the time falls into the current time slice
        if time >= curr_tick {
            let _ = self.summary.increment(value, count);
            let _ = self.histograms[idx].increment(value, count);
        }

        // the duration belonged to a past histogram

        // first we calculated much before current tick the event happened
        let behind = curr_tick.duration_since(time);
        let idx_backward = (behind.as_nanos() / self.resolution.as_nanos()) as usize;

        if idx_backward > self.slices() {
            // We may want to log something ehre
            return;
        }

        let index: usize = if idx_backward > idx {
            idx + self.slices() - idx_backward
        } else {
            idx - idx_backward
        };

        let _ = self.summary.increment(value, count);
        let _ = self.histograms[index].increment(value, count);
    }

    /// Return the nearest value for the requested percentile (0.0 - 100.0)
    /// across the total range of samples retained in the `Heatmap`.
    ///
    /// Note: since the heatmap stores a distribution across a configured time
    /// span, sequential calls to fetch the percentile might result in different
    /// results even without concurrent writers. For instance, you may see a
    /// 90th percentile that is higher than the 100th percentile depending on
    /// the timing of calls to this function and the distribution of your data.
    ///
    /// Note: concurrent writes may also effect the value returned by this
    /// function. Users needing better consistency should ensure that other
    /// threads are not writing into the heatmap while this function is
    /// in-progress.
    pub fn percentile(&self, percentile: f64) -> Result<Bucket, Error> {
        self.tick(Instant::now());
        self.summary.percentile(percentile).map_err(Error::from)
    }

    /// Creates an iterator to iterate over the component histograms of this
    /// heatmap.
    pub fn iter(&self) -> Iter {
        self.into_iter()
    }

    /// Access the summary histogram of this heatmap.
    ///
    /// Note that concurrent modifications to the heatmap will continue to show
    /// up in the summary histogram while it is being read so sequential
    /// queries may not return consistent results.
    pub fn summary(&self) -> &Histogram {
        &self.summary
    }

    // Internal function which handles all the housekeeping tasks that come due
    // as the clock advances- primarily updating the time windows covered by each
    // individual histogram, and cleaning up the values stored in buckets when
    // the histogram is assigned to handle a new time slice.
    fn tick(&self, now: Instant) {
        loop {
            let mut next_tick = self.next_tick.load(Ordering::Relaxed);
            // this is the common case when a heatmap is frequently updated, such as in a busy
            // service.
            if now < next_tick {
                return;
            } else {
                // some expiration needs to happen, let's try to acquire the lock
                //
                // note: we use parking_lot mutex as it will not be poisoned by
                // a thread panic while locked.
                if let Some(_lock) = self.lock.try_lock() {
                    // now that we have the lock, check that we still need to
                    // tick forward.
                    if now < self.next_tick.load(Ordering::Relaxed) {
                        // someone already finished tick maintenance
                        return;
                    }

                    let mut curr_tick = self.curr_tick.load(Ordering::Relaxed);

                    // calculate the number of histogram histogram slices that need cleanup
                    let elapsed = now.checked_duration_since(curr_tick).unwrap();
                    let mut ticks_forward = elapsed.as_nanos() / self.resolution.as_nanos();

                    // move current and next_tick forward
                    curr_tick += Duration::from_nanos(self.resolution.as_nanos() * ticks_forward);
                    next_tick = curr_tick + self.resolution;

                    self.curr_tick.store(curr_tick, Ordering::Relaxed);
                    self.next_tick.store(next_tick, Ordering::Relaxed);

                    // Note that in steady state, the next slice of the previous "current"
                    // histogram (which was marked by `idx`) is the oldest slice of histogram
                    // and therefore needs to be cleaned up. Similarly, if we need to move
                    // the tick forward by more than one window, all histograms we encounter
                    // while advancing the `idx` will need to be cleaned up. Of course, here
                    // the index will be wrapped around the ring buffer if it gets to the end.
                    let mut idx = self.idx.load(Ordering::Relaxed) as usize;
                    while ticks_forward > 0 {
                        idx += 1;
                        if idx == self.histograms.len() {
                            idx = 0;
                        }
                        let _ = self.summary.subtract_and_clear(&self.histograms[idx]);

                        ticks_forward -= 1
                    }

                    self.idx.store(idx, Ordering::Relaxed);
                }
                // if we failed to acquire the lock, just loop. this does mean
                // we busy wait if the heatmap has fallen behind by multiple
                // ticks. we expect the typical case to be that we need to tick
                // forward by just a single slice. in that case, if we fail to
                // acquire the lock, we expect that the loop will terminate when
                // we check `next_tick` at the start of the next iteration.
            }
        }
    }
}

impl Clone for Heatmap {
    fn clone(&self) -> Self {
        let span = self.span;
        let resolution = self.resolution;
        let summary = self.summary.clone();
        let histograms = self.histograms.clone();
        let idx = AtomicUsize::new(self.idx.load(Ordering::Relaxed));
        let create_at = self.create_at;
        let first_tick = self.first_tick;
        let curr_tick = AtomicInstant::new(self.curr_tick.load(Ordering::Relaxed));
        let next_tick = AtomicInstant::new(self.next_tick.load(Ordering::Relaxed));

        Heatmap {
            span,
            resolution,
            summary,
            histograms,
            idx,
            create_at,
            first_tick,
            curr_tick,
            next_tick,
            lock: Mutex::new(()),
        }
    }
}

pub struct Iter<'a> {
    inner: &'a Heatmap,
    index: usize,
    count: usize,
}

impl<'a> Iter<'a> {
    fn new(inner: &'a Heatmap) -> Iter<'a> {
        let index: usize = if inner.active_slices() == inner.slices() {
            inner.idx.load(Ordering::Relaxed) + 1
        } else {
            0
        };
        Iter {
            inner,
            index,
            count: 0,
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a Histogram;

    fn next(&mut self) -> Option<&'a Histogram> {
        if self.count >= self.inner.active_slices() {
            None
        } else {
            let bucket = self.inner.histograms.get(self.index);
            self.index += 1;
            if self.index >= self.inner.slices() {
                self.index = 0;
            }
            self.count += 1;
            bucket
        }
    }
}

impl<'a> IntoIterator for &'a Heatmap {
    type Item = &'a Histogram;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Iter::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn age_out() {
        let heatmap =
            Heatmap::new(0, 4, 20, Duration::from_secs(1), Duration::from_millis(1)).unwrap();
        assert_eq!(heatmap.percentile(0.0).map(|v| v.high()), Err(Error::Empty));
        heatmap.increment(Instant::now(), 1, 1);
        assert_eq!(heatmap.percentile(0.0).map(|v| v.high()), Ok(1));
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert_eq!(heatmap.percentile(0.0).map(|v| v.high()), Ok(1));
        std::thread::sleep(std::time::Duration::from_millis(2000));
        assert_eq!(heatmap.percentile(0.0).map(|v| v.high()), Err(Error::Empty));
    }
}
