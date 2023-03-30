// Copyright 2020 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::Error;
use crate::*;
use core::sync::atomic::*;
use std::cmp::min;

use clocksource::DateTime;
use histogram::{Bucket, Histogram};

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
    create_at: DateTime, // creation time from the (human-readable) systems clock
    first_tick: Instant, // creation time from the monolitic clock

    summary: Histogram,
    histograms: Vec<Histogram>,

    // the instant that the current slice should be "current" until, this is used to
    // look up the correct slice `Histogram` to admit a value/count, as well as to
    // decide when to clean up slices that have aged out
    // Note: we can always compute the correct offset of the `Histogram` slice, as long
    // as we have the beginning of the `Heatmap` which is stored in `first_tick`, and a
    // timestamp which is an `Instant`
    next_tick: AtomicInstant,
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
        let create_at = DateTime::now();
        let first_tick = Instant::now();

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

        let next_tick = AtomicInstant::new(first_tick + resolution);

        Ok(Self {
            span: true_span,
            resolution,
            create_at,
            first_tick,
            summary: Histogram::new(m, r, n)?,
            histograms,
            next_tick,
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

    /// Returns the true span (as `Duration`) that is tracked by the `Heatmap`
    pub fn span(&self) -> Duration {
        self.span
    }

    /// Returns the `Duration` covered by a single slice of histogram
    pub fn resolution(&self) -> Duration {
        self.resolution
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
        let first_tick: u64 = self.first_tick.as_nanos();
        let next_tick = self.next_tick.load(Ordering::Relaxed).as_nanos();
        let total_ticks = ((next_tick - first_tick) / self.resolution.as_nanos()) as usize;
        min(total_ticks, self.slices() - 1)
    }

    /// Returns the `DateTime` representation of when the `Heatmap` was created
    pub fn created_at(&self) -> DateTime {
        self.create_at
    }

    /// Increment a time-value pair by a specified count
    pub fn increment(&self, time: Instant, value: u64, count: u32) -> Result<(), Error> {
        let (next_tick, idx) = self.tick(time);

        let behind = next_tick.duration_since(time);

        // fast path when the time falls into the current time slice
        if behind < self.resolution {
            let _ = self.summary.increment(value, count);
            let _ = self.histograms[idx].increment(value, count);
            return Ok(());
        }

        // the value belonged to a past slice of the histogram

        // first we calculated much before current tick the event happened
        let idx_backward = (behind.as_nanos() / self.resolution.as_nanos()) as usize;

        // span: `slices() - 1`, backward distance: `slices() - 2`
        if idx_backward > self.slices() - 2 {
            return Err(Error::OutOfSpan);
        }

        let index: usize = if idx_backward > idx {
            idx + self.slices() - idx_backward
        } else {
            idx - idx_backward
        };

        let _ = self.summary.increment(value, count);
        let _ = self.histograms[index].increment(value, count);

        Ok(())
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

    // compute the current slice index based on `next_tick` and `first_tick`
    fn slice_idx(&self, next_tick: Instant) -> usize {
        let ntick = next_tick.as_nanos() - self.first_tick.as_nanos() / self.resolution.as_nanos();
        (ntick - 1) as usize % self.slices()
    }

    // Internal function which handles all the housekeeping tasks that come due
    // as the clock advances- primarily updating the time windows covered by each
    // individual histogram, and cleaning up the values stored in buckets when
    // the histogram is assigned to handle a new time slice.
    fn tick(&self, now: Instant) -> (Instant, usize) {
        loop {
            loop {
                let next_tick = self.next_tick.load(Ordering::Relaxed);
                // this is the common case when a heatmap is frequently updated, such as in a busy
                // service.
                if now < next_tick {
                    return (next_tick, self.slice_idx(next_tick));
                }

                let ticks_forward =
                    now.duration_since(next_tick).as_nanos() / self.resolution.as_nanos() + 1;
                let new_tick = Instant::from_nanos(
                    next_tick.as_nanos() + self.resolution.as_nanos() * ticks_forward,
                );
                let result = self.next_tick.compare_exchange(
                    next_tick,
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
                        // `next_tick` and `new_tick`, i.e. we start from 2 slices to the right of
                        //  `next_tick`, and stop until we finish clearing slice immediately to
                        // the right of `new_tick`. When `tick()` is called frequently enough, we
                        // can assume `new_tick` is just to the right of `next_tick`, and the
                        // clearing is performed on exactly one slice, which is the slice
                        // immediately to the right of `new_tick` and 2 over to the right of
                        // `next_tick`. (The slice immediately to the right of `next_tick` which is
                        // `new_tick` now points to as current was already cleared previously, due
                        // to having a buffer slice.)
                        //
                        // When the `next_tick` falls further behind, i.e. we need to clear more
                        // than one slice, the buffer slice allocated can no longer guarantee that
                        // new `increment`s are against a cleared slice, and reported `summary` may
                        // be incorrect. We will clear all slices that should be cleared, but /
                        // do not attempt to correct the staleness or inconsistencies in reporting.
                        //
                        // We may revisit this decision in the future.
                        let mut idx = self.slice_idx(next_tick) + 1;
                        for _ in 0..ticks_forward {
                            idx += 1;
                            if idx >= self.slices() {
                                idx -= self.slices();
                            }
                            let _ = self.summary.subtract_and_clear(&self.histograms[idx]);
                        }
                        return (new_tick, self.slice_idx(new_tick));
                    }
                }
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
        let create_at = self.create_at;
        let first_tick = self.first_tick;
        let next_tick = AtomicInstant::new(first_tick + resolution);

        Heatmap {
            span,
            resolution,
            create_at,
            first_tick,
            summary,
            histograms,
            next_tick,
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
            inner.slice_idx(inner.next_tick.load(Ordering::Relaxed)) + 2
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
        heatmap.increment(Instant::now(), 1, 1).unwrap();
        assert_eq!(heatmap.percentile(0.0).map(|v| v.high()), Ok(1));
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert_eq!(heatmap.percentile(0.0).map(|v| v.high()), Ok(1));
        std::thread::sleep(std::time::Duration::from_millis(2000));
        assert_eq!(heatmap.percentile(0.0).map(|v| v.high()), Err(Error::Empty));
    }
}
