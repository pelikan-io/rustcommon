// Copyright 2021 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

//! Methods and structs for working with dynamically created and destroyed
//! metrics.
//!
//! Generally users should not need to use anything in this module with the
//! exception of [`DynPinnedMetric`] and [`DynBoxedMetric`].

use std::borrow::Cow;
use std::ops::Deref;
use std::pin::Pin;

use crate::WrapMetric;
use crate::{Format, Metric, MetricEntry};

/// Builder for creating a dynamic metric.
///
/// This can be used to directly create a [`DynBoxedMetric`] or you can convert
/// this builder into a [`MetricEntry`] for more advanced use cases.
pub struct MetricBuilder(metriken_core::dynmetrics::MetricBuilder);

impl MetricBuilder {
    /// Create a new builder, starting with the metric name.
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self(metriken_core::dynmetrics::MetricBuilder::new(name.into()))
    }

    /// Add a description of this metric.
    pub fn description(self, desc: impl Into<Cow<'static, str>>) -> Self {
        Self(self.0.description(desc))
    }

    /// Add a new key-value metadata entry.
    pub fn metadata(self, key: impl Into<String>, value: impl Into<String>) -> Self {
        Self(self.0.metadata(key, value))
    }

    pub fn formatter(self, formatter: fn(&MetricEntry, Format) -> String) -> Self {
        // SAFETY: MetricEntry is #[repr(transparent)] around metriken_core::MetricEntry
        //         so implicitly transmuting their pointers as part of a function call is
        //         safe.
        let translated: fn(&metriken_core::MetricEntry, Format) -> String =
            unsafe { std::mem::transmute(formatter) };
        Self(self.0.formatter(translated))
    }

    /// Convert this builder directly into a [`MetricEntry`].
    pub fn into_entry(self) -> MetricEntry {
        MetricEntry(self.0.into_entry())
    }

    /// Build a [`DynBoxedMetric`] for use with this builder.
    pub fn build<T: Metric>(self, metric: T) -> DynBoxedMetric<T> {
        DynBoxedMetric::new(metric, self.into_entry())
    }
}

/// A dynamic metric that stores the metric inline.
///
/// This is a dynamic metric that relies on pinning guarantees to ensure that
/// the stored metric can be safely accessed from other threads looking through
/// the global dynamic metrics registry. As it requires pinning, it is somewhat
/// unweildy to use. Most use cases can probably use [`DynBoxedMetric`] instead.
///
/// To use this, first create the `DynPinnedMetric` and then, once it is pinned,
/// call [`register`] any number of times with all of the names the metric
/// should be registered under. When the `DynPinnedMetric` instance is dropped
/// it will unregister all the metric entries added via [`register`].
///
/// # Example
/// ```
/// # use metriken::*;
/// # use std::pin::pin;
/// let my_dyn_metric = pin!(DynPinnedMetric::new(Counter::new()));
/// my_dyn_metric.as_ref().register(MetricBuilder::new("a.dynamic.counter").into_entry());
/// ```
///
/// [`register`]: crate::dynmetrics::DynPinnedMetric::register
pub struct DynPinnedMetric<M: Metric>(metriken_core::dynmetrics::DynPinnedMetric<WrapMetric<M>>);

impl<M: Metric> DynPinnedMetric<M> {
    /// Create a new `DynPinnedMetric` with the provided internal metric.
    ///
    /// This does not register the metric. To do that call [`register`].
    ///
    /// [`register`]: self::DynPinnedMetric::register
    pub fn new(metric: M) -> Self {
        Self(metriken_core::dynmetrics::DynPinnedMetric::new(
            WrapMetric::new(metric),
        ))
    }

    /// Register this metric in the global list of dynamic metrics with `name`.
    ///
    /// Calling this multiple times will result in the same metric being
    /// registered multiple times under potentially different names.
    pub fn register(self: Pin<&Self>, entry: MetricEntry) {
        // SAFETY: This is run-of-the-mill pin projection.
        let this = unsafe { self.map_unchecked(|this| &this.0) };
        this.register(entry.0);
    }
}

impl<M: Metric> Deref for DynPinnedMetric<M> {
    type Target = M;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A dynamic metric that stores the metric instance on the heap.
///
/// This avoids a lot of the hangup with [`DynPinnedMetric`] as it allows for
/// moving the `DynBoxedMetric` without having to worry about pinning or safety
/// issues. However, this comes at the expense of requiring a heap allocation
/// for the metric.
///
/// # Example
/// ```
/// # use metriken::*;
/// let my_gauge = MetricBuilder::new("my.dynamic.gauge").build(Gauge::new());
///
/// my_gauge.increment();
/// ```
pub struct DynBoxedMetric<M: Metric> {
    metric: Pin<Box<DynPinnedMetric<M>>>,
}

impl<M: Metric> DynBoxedMetric<M> {
    /// Create a new dynamic metric using the provided metric type with the
    /// provided `name`.
    pub fn new(metric: M, entry: MetricEntry) -> Self {
        let this = Self::unregistered(metric);
        this.register(entry);
        this
    }

    /// Create a new dynamic metric without registering it.
    fn unregistered(metric: M) -> Self {
        Self {
            metric: Box::pin(DynPinnedMetric::new(metric)),
        }
    }

    /// Register this metric in the global list of dynamic metrics with `name`.
    ///
    /// Calling this multiple times will result in the same metric being
    /// registered multiple times under potentially different names.
    fn register(&self, entry: MetricEntry) {
        self.metric.as_ref().register(entry)
    }
}

impl<M: Metric> Deref for DynBoxedMetric<M> {
    type Target = M;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.metric
    }
}
