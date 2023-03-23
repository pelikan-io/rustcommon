// Copyright 2020 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::AtomicInstant;
use clocksource::*;
use core::sync::atomic::Ordering;
use histogram::Histogram;

pub struct Window<'a> {
    pub(crate) start: Instant<Nanoseconds<u64>>,
    pub(crate) stop: Instant<Nanoseconds<u64>>,
    pub(crate) histogram: &'a Histogram,
}

pub(crate) struct OwnedWindow {
    pub(crate) start: AtomicInstant,
    pub(crate) stop: AtomicInstant,
    pub(crate) histogram: Histogram,
}

impl Clone for OwnedWindow {
    fn clone(&self) -> Self {
        Self {
            start: AtomicInstant::new(self.start.load(Ordering::Relaxed)),
            stop: AtomicInstant::new(self.stop.load(Ordering::Relaxed)),
            histogram: self.histogram.clone(),
        }
    }
}

impl<'a> Window<'a> {
    pub fn start(&self) -> Instant<Nanoseconds<u64>> {
        self.start
    }

    pub fn stop(&self) -> Instant<Nanoseconds<u64>> {
        self.stop
    }

    pub fn histogram(&self) -> &'a Histogram {
        self.histogram
    }
}
