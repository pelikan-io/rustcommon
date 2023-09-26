// Copyright 2020 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::Error;
use crate::*;
use core::sync::atomic::*;
use std::cmp::min;

pub use histogram::{Bucket, Histogram, Percentile};

type UnixInstant = clocksource::UnixInstant<Nanoseconds<u64>>;

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
///
///
pub struct Heatmap {
    // these two fields are only set at creation time
    span: Duration,
    resolution: Duration,
    // the following two timestamps are treated as equivalent even though they are
    // not strictly so. This is so that we have a reference point to convert elapsed
    // time to wall clock time (e.g. used in `Waterfall`)
    start_ts: UnixInstant, // reading of the systems time (e.g. from `CLOCK_REALTIME`)
    tick_origin: Instant,  // reference point (on the monolitic clock) of the `Heatmap`

    summary: Histogram,
    histograms: Vec<Histogram>,

    // the instant that the current slice should be "current" until, this is used to
    // look up the correct slice `Histogram` to admit a value/count, as well as to
    // decide when to clean up slices that have aged out
    // Note: we can always compute the correct offset of the `Histogram` slice, as long
    // as we have the beginning of the `Heatmap` which is stored in `tick_origin`, and a
    // timestamp which is an `Instant`
    tick_at: AtomicInstant,
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
    // align tick boundary in `Heatmap` with `resolution`
    align: bool,
}

impl Builder {
    /// Consume the `Builder` and return a `Heatmap`.
    pub fn build(self) -> Result<Heatmap, Error> {
        // best effort attempt to line up start time with a system clock boundary
        // decided by `resolution`
        if self.align {
            // get the remainder against resolution
            let monotonic_now = Instant::now();
            let system_now = UnixInstant::now();
            let delta = Duration::from_nanos(
                system_now
                    .duration_since(UnixInstant::from_nanos(0))
                    .as_nanos()
                    % self.resolution.as_nanos(),
            );
            Heatmap::new(
                self.m,
                self.r,
                self.n,
                self.span,
                self.resolution,
                system_now.checked_sub(delta),
                monotonic_now.checked_sub(delta),
            )
        } else {
            Heatmap::new(
                self.m,
                self.r,
                self.n,
                self.span,
                self.resolution,
                None,
                None,
            )
        }
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

    /// Align start for the resolution give. If resolution is secondly, setting
    /// `align` to true will lead to a best-effort attempt to put `tick_origin`
    /// value at or near the latest multiple of `resolution` in the past.
    pub fn align(mut self, align: bool) -> Self {
        self.align = align;
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
    ///
    /// - `start_ts` - the `UnixInstant` the `Heatmap` should have started its clock.
    /// - `start_instant` - the `Instant` the `Heatmap` should have started its clock.
    /// Both should be provided or skipped. When both are skipped, the function will
    /// take the current reading from the system real-time clock and monotonic clock.
    /// When provided, the caller can set them to different values than the current
    /// time, e.g. set the `start_ts` (and providing corresponding `start_instant`) as
    /// the top of second.
    ///
    pub fn new(
        m: u32,
        r: u32,
        n: u32,
        span: Duration,
        resolution: Duration,
        start_ts: Option<UnixInstant>,
        start_instant: Option<Instant>,
    ) -> Result<Self, Error> {
        let mut histograms = Vec::new();
        let mut true_span = Duration::from_nanos(0);
        let mut span_stop = span;
        let start_ts = start_ts.unwrap_or(UnixInstant::now());
        let tick_origin = start_instant.unwrap_or(Instant::now());

        // `true_span` is always a multiple of `resolution`, which means it maybe greater than
        // the `span` provided by the `Builder`.
        // Assuming `true_span / resolution = n`, we allocate `n + 1` `Histograms` so we always
        // have a cleared one in the ring
        span_stop += resolution;
        while true_span < span_stop {
            histograms.push(Histogram::new(m, r, n).unwrap());
            true_span += resolution;
        }
        histograms.shrink_to_fit();

        let tick_at = AtomicInstant::new(tick_origin + resolution);

        Ok(Self {
            span: true_span,
            resolution,
            start_ts,
            tick_origin,
            summary: Histogram::new(m, r, n)?,
            histograms,
            tick_at,
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
            align: false,
        }
    }

    /// Returns the true span (as `Duration`) that is tracked by the `Heatmap`
    pub fn span(&self) -> Duration {
        self.span
    }

    /// Returns the `Duration` covered by a single slice of histogram
    pub fn resolution(&self) -> Duration {
        self.resolution
    }

    /// Returns the `start_at` timestamp of the `Heatmap`
    pub fn start_at(&self) -> UnixInstant {
        self.start_ts
    }

    /// Returns the number of `Histogram` slices the `Heatmap` can hold, each covering `resolution`
    /// Note we allocate one more slice than what `span` demands
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
        let instant = self.tick_at.load(Ordering::Relaxed);
        let elapsed = instant.duration_since(self.tick_origin);
        let total_ticks = (elapsed.as_nanos() / self.resolution.as_nanos()) as usize;
        min(total_ticks, self.slices() - 1)
    }

    /// Increment a time-value pair by a specified count
    pub fn increment(&self, time: Instant, value: u64, count: u32) -> Result<(), Error> {
        let (tick_at, mut idx, ntick) = self.tick(time);

        let behind = tick_at.duration_since(time);

        if behind > self.resolution {
            // the value belonged to a past slice of the histogram, should be an uncommon path

            // first we calculated much before current tick the event happened
            let idx_backward = (behind.as_nanos() / self.resolution.as_nanos()) as usize;

            if idx_backward > self.active_slices() - 1 {
                return Err(Error::OutOfSpan);
            }

            idx = self.idx_delta(idx, -(idx_backward as i64));
        }

        self.summary.increment(value, count)?;
        self.histograms[idx].increment(value, count)?;
        if ntick <= 1 {
            Ok(())
        } else {
            Err(Error::StaleClock)
        }
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
    pub fn percentiles(&self, percentiles: &[f64]) -> Result<Vec<Percentile>, Error> {
        self.tick(Instant::now());
        self.summary.percentiles(percentiles).map_err(Error::from)
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

    fn idx_delta(&self, idx: usize, delta: i64) -> usize {
        (idx + (self.slices() as i64 + delta) as usize) % self.slices()
    }

    // compute the current slice index based on `tick_at` and `tick_origin`
    fn slice_idx(&self, tick_at: Instant) -> usize {
        let ntick =
            tick_at.duration_since(self.tick_origin).as_nanos() / self.resolution.as_nanos();
        (ntick - 1) as usize % self.slices()
    }

    // Internal function which handles all the housekeeping tasks that come due
    // as the clock advances- primarily updating the time windows covered by each
    // individual histogram, and cleaning up the values stored in buckets when
    // the histogram is assigned to handle a new time slice.
    //
    // It returns the `tick_at` value which indicates the upper bound of the
    // current `span`, the index of the most recent `Histogram` slice, and by
    // how many ticks the heatmap moved forward.
    fn tick(&self, now: Instant) -> (Instant, usize, usize) {
        loop {
            let tick_at = self.tick_at.load(Ordering::Relaxed);
            // this is the common case when a heatmap is frequently updated, such as in a busy
            // service.
            if now < tick_at {
                return (tick_at, self.slice_idx(tick_at), 0);
            }

            let ticks_forward =
                now.duration_since(tick_at).as_nanos() / self.resolution.as_nanos() + 1;
            let mut new_tick = self.tick_at.load(Ordering::Relaxed);
            for _ in 0..ticks_forward {
                new_tick += self.resolution;
            }
            let result = self.tick_at.compare_exchange(
                tick_at,
                new_tick,
                Ordering::AcqRel,
                Ordering::Relaxed,
            );
            match result {
                Err(_) => {
                    // We will loop back to the top and see if the newly stored value is current
                    // or we still need to move the tick forward
                }
                Ok(_) => {
                    // clean up `Histogram` slices if needed, then return
                    //
                    // because the `compare_exchange` operation above was successful, we know
                    // we have exclusive clean up access to the slices covered between the
                    // `tick_at` and `new_tick`, i.e. we start from 2 slices to the right of
                    //  `tick_at`, and stop until we finish clearing slice immediately to
                    // the right of `new_tick`. When `tick()` is called frequently enough, we
                    // can assume `new_tick` is just to the right of `tick_at`, and the
                    // clearing is performed on exactly one slice, which is the slice
                    // immediately to the right of `new_tick` and 2 over to the right of
                    // `tick_at`. (The slice immediately to the right of `tick_at` which is
                    // `new_tick` now points to as current was already cleared previously, due
                    // to having a buffer slice.)
                    //
                    // When the `tick_at` falls further behind, i.e. we need to clear more
                    // than one slice, the buffer slice allocated can no longer guarantee that
                    // new `increment`s are against a cleared slice, and reported `summary` may
                    // be incorrect. We will clear all slices that should be cleared, but /
                    // do not attempt to correct the staleness or inconsistencies in reporting.
                    //
                    // We may revisit this decision in the future.
                    let mut idx = self.idx_delta(self.slice_idx(tick_at), 1);
                    for _ in 0..ticks_forward {
                        idx = self.idx_delta(idx, 1);
                        let _ = self.summary.subtract_and_clear(&self.histograms[idx]);
                    }
                    return (new_tick, self.slice_idx(new_tick), ticks_forward as usize);
                }
            }
        }
    }
}

impl Clone for Heatmap {
    fn clone(&self) -> Self {
        let span = self.span;
        let resolution = self.resolution;
        let start_ts = self.start_ts;
        let tick_origin = self.tick_origin;
        let summary = self.summary.clone();
        let histograms = self.histograms.clone();
        let tick_at = AtomicInstant::new(tick_origin + resolution);

        Heatmap {
            span,
            resolution,
            start_ts,
            tick_origin,
            summary,
            histograms,
            tick_at,
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
        let index: usize = if inner.active_slices() == inner.slices() - 1 {
            // When the `Histogram` vector is fully utilized, the current slice is on the left
            // (factoring in wraparound) of the buffer slice, which is on the left of the oldest
            // slice
            inner.idx_delta(inner.slice_idx(inner.tick_at.load(Ordering::Relaxed)), 2)
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
        let heatmap = Heatmap::new(
            0,
            4,
            20,
            Duration::from_secs(1),
            Duration::from_millis(1),
            None,
            None,
        )
        .unwrap();
        assert_eq!(heatmap.percentile(0.0).map(|v| v.high()), Err(Error::Empty));
        heatmap.increment(Instant::now(), 1, 1).unwrap();
        assert_eq!(heatmap.percentile(0.0).map(|v| v.high()), Ok(1));
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert_eq!(heatmap.percentile(0.0).map(|v| v.high()), Ok(1));
        std::thread::sleep(std::time::Duration::from_millis(2000));
        assert_eq!(heatmap.percentile(0.0).map(|v| v.high()), Err(Error::Empty));
    }
}
