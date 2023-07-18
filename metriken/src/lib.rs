// Copyright 2021 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

//! Easily registered distributed metrics.
//!
//! More docs todo...
//!
//! # Creating a Metric
//! Registering a metric is straightforward. All that's needed is to declare a
//! static within the [`metric`] macro. By default, the metric will have the
//! name of the path to the static variable you used to declare it but this can
//! be overridden by passing the `name` parameter to the macro.
//!
//! ```
//! # // This should remain in sync with the example below.
//! use metriken::*;
//! /// A counter metric named "my.metric.name"
//! #[metric(name = "my.metric.name")]
//! static COUNTER_B: Counter = Counter::new();
//!
//! /// A counter metric named "<crate name>::COUNTER_A"
//! #[metric]
//! static COUNTER_A: Counter = Counter::new();
//! #
//! # let metrics = metrics();
//! # // Metrics may be in any arbitrary order
//! # let mut names: Vec<_> = metrics.iter().map(|metric| metric.name()).collect();
//! # names.sort();
//! #
//! # assert_eq!(names.len(), 2);
//! # assert_eq!(names[0], "COUNTER_A");
//! # assert_eq!(names[1], "my.metric.name");
//! ```
//!
//! # Accessing Metrics
//! All metrics registered via the [`metric`] macro can be accessed by calling
//! the [`metrics`] function. This will return an instance of the [`Metric`]
//! struct which allows you to access all staticly and dynamically registered
//! metrics.
//!
//! Suppose we have the metrics declared in the example above.
//! ```
//! # // This should remain in sync with the example above.
//! # use metriken::*;
//! # /// A counter metric named "my.metric.name"
//! # #[metric(name = "my.metric.name")]
//! # static COUNTER_B: Counter = Counter::new();
//! #
//! # /// A counter metric named "<crate name>::COUNTER_A"
//! # #[metric]
//! # static COUNTER_A: Counter = Counter::new();
//! #
//! let metrics = metrics();
//! // Metrics may be in any arbitrary order
//! let mut names: Vec<_> = metrics.iter().map(|metric| metric.name()).collect();
//! names.sort();
//!
//! assert_eq!(names.len(), 2);
//! assert_eq!(names[0], "COUNTER_A");
//! assert_eq!(names[1], "my.metric.name");
//! ```
//!
//! # How it Works
//! Behind the scenes, this crate uses the [`linkme`] crate to create a
//! distributed slice containing a [`MetricEntry`] instance for each metric that
//! is registered via the [`metric`] attribute.

use std::any::Any;
use std::borrow::Cow;

use parking_lot::RwLockReadGuard;

use crate::export::MetricWrapper;

mod counter;
mod gauge;
mod heatmap;
mod impls;
mod lazy;
mod metadata;
mod null;

extern crate self as metriken;

pub mod dynmetrics;

pub use crate::counter::Counter;
pub use crate::dynmetrics::{DynBoxedMetric, DynPinnedMetric, MetricBuilder};
pub use crate::gauge::Gauge;
pub use crate::heatmap::Heatmap;
pub use crate::lazy::Lazy;
pub use crate::metadata::{Metadata, MetadataIter};

pub use metriken_derive::metric;

#[doc(hidden)]
pub mod export {
    use crate::{Metric, MetricEntry, FormatFn};

    pub extern crate linkme;
    pub extern crate phf;

    /// You can't use `dyn <trait>s` directly in const methods for now but a wrapper
    /// is fine. This wrapper is a work around to allow us to use const constructors
    /// for the MetricEntry struct.
    #[doc(hidden)]
    pub struct MetricWrapper(pub *const dyn Metric);

    #[linkme::distributed_slice]
    pub static METRICS: [MetricEntry] = [..];

    pub const fn entry(
        metric: &'static dyn Metric,
        name: &'static str,
        description: Option<&'static str>,
        metadata: &'static phf::Map<&'static str, &'static str>,
        formatter: Option<FormatFn>,
    ) -> MetricEntry {
        use std::borrow::Cow;

        MetricEntry {
            metric: MetricWrapper(metric),
            name: Cow::Borrowed(name),
            description: match description {
                Some(desc) => Some(Cow::Borrowed(desc)),
                None => None,
            },
            metadata: crate::Metadata::new_static(metadata),
            formatter,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Format {
    Plain,
    Prometheus,
}

pub type FormatFn = fn(&MetricEntry, Format) -> String;

/// A counter holds a unsigned 64bit monotonically non-decreasing value. The
/// counter behavior is to wrap on overflow.
///
/// Common examples are the number of operations (requests, reads, ...) or
/// errors.
///
/// Unlike a standard `Counter`, a `LazyCounter` will not report a value unless
/// it has been initialized by writing to at least once. This is useful for when
/// you want to declare metrics statically, but only report metrics that are
/// being used.
pub type LazyCounter = Lazy<Counter>;

/// A gauge holds a signed 64-bit value and is used to represent metrics which
/// may increase or decrease in value. The behavior is to wrap around on
/// overflow and underflow.
///
/// Common examples are queue depths, temperatures, and usage metrics.
///
/// Unlike a standard `Gauge`, a `LazyGauge` will not report a value unless it
/// has been initialized by writing to at least once. This is useful for when
/// you want to declare metrics statically, but only report metrics that are
/// being used.
pub type LazyGauge = Lazy<Gauge>;

/// Global interface to a metric.
///
/// Most use of metrics should use the directly declared constants.
pub trait Metric: Send + Sync + 'static {
    /// Indicate whether this metric has been set up.
    ///
    /// Generally, if this returns `false` then the other methods on this
    /// trait should return `None`.
    fn is_enabled(&self) -> bool {
        self.as_any().is_some()
    }

    /// Get the current metric as an [`Any`] instance. This is meant to allow
    /// custom processing for known metric types.
    ///
    /// [`Any`]: std::any::Any
    fn as_any(&self) -> Option<&dyn Any>;
}

/// A statically declared metric entry.
pub struct MetricEntry {
    metric: MetricWrapper,
    name: Cow<'static, str>,
    description: Option<Cow<'static, str>>,
    metadata: Metadata,
    formatter: Option<FormatFn>,
}

impl MetricEntry {
    /// Get a reference to the metric that this entry corresponds to.
    pub fn metric(&self) -> &dyn Metric {
        unsafe { &*self.metric.0 }
    }

    /// Get the name of this metric.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the description of this metric.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn formatter(&self) -> Option<FormatFn> {
        self.formatter
    }

    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }
}

unsafe impl Send for MetricEntry {}
unsafe impl Sync for MetricEntry {}

impl std::ops::Deref for MetricEntry {
    type Target = dyn Metric;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.metric()
    }
}

impl std::fmt::Debug for MetricEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MetricEntry")
            .field("name", &self.name())
            .field("metric", &"<dyn Metric>")
            .finish()
    }
}

/// The list of all metrics registered via the either [`metric`] attribute or by
/// using the types within the [`dynmetrics`] module.
///
/// Names within metrics are not guaranteed to be unique and no aggregation of
/// metrics with the same name is done.
pub fn metrics() -> Metrics {
    Metrics {
        dyn_metrics: crate::dynmetrics::get_registry(),
    }
}

/// Provides access to all registered metrics both static and dynamic.
///
/// **IMPORTANT:** Note that while any instance of this struct is live
/// attempting to register or unregister any dynamic metrics will block.
/// If this is done on the same thread as is currently working with an instance
/// of `Metrics` then it will cause a deadlock. If your application will be
/// registering and unregistering dynamic metrics then you should avoid holding
/// on to `Metrics` instances for long periods of time.
///
/// `Metrics` instances can be created via the [`metrics`] function.
pub struct Metrics {
    dyn_metrics: RwLockReadGuard<'static, dynmetrics::DynMetricsRegistry>,
}

impl Metrics {
    /// A list containing all metrics that were registered via the [`metric`]
    /// attribute macro.
    pub fn static_metrics(&self) -> &'static [MetricEntry] {
        &crate::export::METRICS
    }

    /// A list containing all metrics that were dynamically registered.
    pub fn dynamic_metrics(&self) -> &[MetricEntry] {
        self.dyn_metrics.metrics()
    }

    pub fn iter(&self) -> <&Self as IntoIterator>::IntoIter {
        self.into_iter()
    }
}

impl<'a> IntoIterator for &'a Metrics {
    type Item = &'a MetricEntry;

    type IntoIter =
        std::iter::Chain<std::slice::Iter<'a, MetricEntry>, std::slice::Iter<'a, MetricEntry>>;

    fn into_iter(self) -> Self::IntoIter {
        self.static_metrics()
            .iter()
            .chain(self.dynamic_metrics().iter())
    }
}
