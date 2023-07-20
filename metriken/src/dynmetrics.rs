// Copyright 2021 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

//! Methods and structs for working with dynamically created and destroyed
//! metrics.
//!
//! Generally users should not need to use anything in this module with the
//! exception of [`DynPinnedMetric`] and [`DynBoxedMetric`].

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::marker::PhantomPinned;
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::pin::Pin;

use crate::null::NullMetric;
use crate::{Metadata, Metric, MetricEntry, MetricWrapper};

// We use parking_lot here since it avoids lock poisioning
use parking_lot::{const_rwlock, RwLock, RwLockReadGuard};

pub(crate) struct DynMetricsRegistry {
    metrics: BTreeMap<usize, MetricEntry>,
}

impl DynMetricsRegistry {
    const fn new() -> Self {
        Self {
            metrics: BTreeMap::new(),
        }
    }

    fn key_for(entry: &MetricEntry) -> usize {
        entry.metric() as *const dyn Metric as *const () as usize
    }

    fn register(&mut self, entry: MetricEntry) {
        self.metrics.insert(Self::key_for(&entry), entry);
    }

    fn unregister(&mut self, metric: *const dyn Metric) {
        let key = metric as *const () as usize;
        self.metrics.remove(&key);
    }

    pub(crate) fn metrics(&self) -> &BTreeMap<usize, MetricEntry> {
        &self.metrics
    }
}

static REGISTRY: RwLock<DynMetricsRegistry> = const_rwlock(DynMetricsRegistry::new());

pub(crate) fn get_registry() -> RwLockReadGuard<'static, DynMetricsRegistry> {
    REGISTRY.read()
}

/// Builder for creating a dynamic metric.
///
/// This can be used to directly create a [`DynBoxedMetric`] or you can convert
/// this builder into a [`MetricEntry`] for more advanced use cases.
pub struct MetricBuilder {
    name: Cow<'static, str>,
    desc: Option<Cow<'static, str>>,
    metadata: HashMap<String, String>,
}

impl MetricBuilder {
    /// Create a new builder, starting with the metric name.
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: name.into(),
            desc: None,
            metadata: HashMap::new(),
        }
    }

    /// Add a description of this metric.
    pub fn description(mut self, desc: impl Into<Cow<'static, str>>) -> Self {
        self.desc = Some(desc.into());
        self
    }

    /// Add a new key-value metadata entry.
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Convert this builder directly into a [`MetricEntry`].
    pub fn into_entry(self) -> MetricEntry {
        MetricEntry {
            metric: MetricWrapper(&NullMetric),
            name: self.name,
            description: self.desc,
            metadata: Metadata::new(self.metadata),
        }
    }

    /// Build a [`DynBoxedMetric`] for use with this builder.
    pub fn build<T: Metric>(self, metric: T) -> DynBoxedMetric<T> {
        DynBoxedMetric::new(metric, self.into_entry())
    }
}

/// Registers a new dynamic metric entry.
///
/// The [`MetricEntry`] instance will be kept until an [`unregister`] call is
/// made with a metric pointer that matches the one within the [`MetricEntry`].
/// When using this take care to note how it interacts with [`MetricEntry`]'s
/// safety guarantees.
///
/// # Safety
/// The pointer in `entry.metric` must remain valid to dereference until it is
/// removed from the registry via [`unregister`].
pub(crate) unsafe fn register(entry: MetricEntry) {
    REGISTRY.write().register(entry);
}

/// Unregisters all dynamic entries added via [`register`] that point to the
/// same address as `metric`.
///
/// This function may remove multiple entries if the same metric has been
/// registered multiple times.
pub(crate) fn unregister(metric: *const dyn Metric) {
    REGISTRY.write().unregister(metric);
}

/// Ensures that the metric `M` has a unique address.
///
/// The correctness of the registry depends on each dynamic address having a
/// unique address. However, we don't want to unconditionally add padding to
/// all metrics. The way to work around this is to union M with a type of size
/// 1. That way, if M is a zero-sized type then the storage will have a size
/// of 1 but otherwise it has the size of M.
union PinnedMetricStorage<M> {
    metric: ManuallyDrop<M>,
    _padding: u8,
}

impl<M> PinnedMetricStorage<M> {
    fn new(metric: M) -> Self {
        Self {
            metric: ManuallyDrop::new(metric),
        }
    }

    #[inline]
    fn metric(&self) -> &M {
        // Safety: nothing ever accesses _padding
        unsafe { &self.metric }
    }
}

impl<M> Drop for PinnedMetricStorage<M> {
    fn drop(&mut self) {
        unsafe { ManuallyDrop::drop(&mut self.metric) }
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
pub struct DynPinnedMetric<M: Metric> {
    storage: PinnedMetricStorage<M>,
    // This type relies on Pin's guarantees for correctness. Allowing it to be unpinned would cause
    // errors.
    _marker: PhantomPinned,
}

impl<M: Metric> DynPinnedMetric<M> {
    /// Create a new `DynPinnedMetric` with the provided internal metric.
    ///
    /// This does not register the metric. To do that call [`register`].
    ///
    /// [`register`]: self::DynPinnedMetric::register
    pub fn new(metric: M) -> Self {
        Self {
            storage: PinnedMetricStorage::new(metric),
            _marker: PhantomPinned,
        }
    }

    /// Register this metric in the global list of dynamic metrics with `name`.
    ///
    /// Calling this multiple times will result in the same metric being
    /// registered multiple times under potentially different names.
    pub fn register(self: Pin<&Self>, mut entry: MetricEntry) {
        entry.metric = MetricWrapper(self.storage.metric());

        // SAFETY:
        // To prove that this is safe we need to list out a few guarantees/requirements:
        //  - Pin ensures that the memory of this struct instance will not be reused
        //    until the drop call completes.
        //  - MetricEntry::new_unchecked requires that the metric reference outlive
        //    created the MetricEntry instance.
        //
        // Finally, register will keep the MetricEntry instance in a global list until
        // the corresponding unregister call is made.
        //
        // Taking all of these together, we can guarantee that self.metric will not be
        // dropped until this instance of DynPinnedMetric is dropped itself. At that
        // point, drop calls unregister which will drop the MetricEntry instance. This
        // ensures that the references to self.metric in REGISTRY will always be valid
        // and that this method is safe.
        unsafe { register(entry) };
    }
}

impl<M: Metric> Drop for DynPinnedMetric<M> {
    fn drop(&mut self) {
        // If this metric has not been registered then nothing will be removed.
        unregister(self.storage.metric());
    }
}

impl<M: Metric> Deref for DynPinnedMetric<M> {
    type Target = M;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.storage.metric()
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
