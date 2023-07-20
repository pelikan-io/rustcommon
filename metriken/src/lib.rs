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
//! /// A counter metric named "<crate name>::COUNTER_A"
//! #[metric]
//! static COUNTER_A: Counter = Counter::new();
//!
//! /// A counter metric named "my.metric.name"
//! #[metric(name = "my.metric.name")]
//! static COUNTER_B: Counter = Counter::new();
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
//! # /// A counter metric named "<crate name>::COUNTER_A"
//! # #[metric]
//! # static COUNTER_A: Counter = Counter::new();
//! #
//! # /// A counter metric named "my.metric.name"
//! # #[metric(name = "my.metric.name")]
//! # static COUNTER_B: Counter = Counter::new();
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

use crate::export::MetricWrapper;

/// A helper macro for marking imports as being used.
///
/// This is meant to be used for when a reference is made to an item from a doc
/// comment but that item isn't actually used for code anywhere.
macro_rules! used_in_docs {
    ($($item:ident),* $(,)?) => {
        const _: () = {
            #[allow(unused_imports)]
            mod _docs {
                $( use super::$item; )*
            }
        };
    };
}

mod counter;
mod gauge;
mod heatmap;
mod lazy;
mod metadata;
mod metrics;
mod null;

extern crate self as metriken;

pub mod dynmetrics;

pub use crate::counter::Counter;
pub use crate::dynmetrics::{DynBoxedMetric, DynPinnedMetric, MetricBuilder};
pub use crate::gauge::Gauge;
pub use crate::heatmap::Heatmap;
pub use crate::lazy::Lazy;
pub use crate::metadata::{Metadata, MetadataIter};
pub use crate::metrics::{metrics, DynMetricsIter, Metrics, MetricsIter};

pub use metriken_derive::metric;

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

#[doc(hidden)]
pub mod export {
    pub extern crate linkme;
    pub extern crate phf;

    use crate::{Metadata, Metric};

    /// You can't use `dyn <trait>s` directly in const methods for now but a wrapper
    /// is fine. This wrapper is a work around to allow us to use const constructors
    /// for the MetricEntry struct.
    pub struct MetricWrapper(pub *const dyn Metric);

    #[linkme::distributed_slice]
    pub static METRICS: [crate::MetricEntry] = [..];

    pub const fn entry(
        metric: &'static dyn crate::Metric,
        name: &'static str,
        description: Option<&'static str>,
        metadata: &'static phf::Map<&'static str, &'static str>,
    ) -> crate::MetricEntry {
        use std::borrow::Cow;

        crate::MetricEntry {
            metric: MetricWrapper(metric),
            name: Cow::Borrowed(name),
            description: match description {
                Some(desc) => Some(Cow::Borrowed(desc)),
                None => None,
            },
            metadata: Metadata::new_static(metadata),
        }
    }
}

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

    /// Get the value of the current metric, should it be enabled.
    ///
    /// # Note to Implementors
    /// If your metric's value does not correspond to one of the variants of
    /// [`Value`] then return [`Value::Other`] and metric consumers can use
    /// [`as_any`](crate::Metric::as_any) to specifically handle your metric.
    fn value(&self) -> Option<Value>;
}

/// The value of a metric.
///
/// See [`Metric::value`].
#[non_exhaustive]
#[derive(Clone)]
pub enum Value<'a> {
    /// A counter value.
    Counter(u64),

    /// A gauge value.
    Gauge(i64),

    Heatmap(&'a Heatmap),

    /// The value of the metric could not be represented using the other
    /// `Value` variants.
    ///
    /// Use [`Metric::as_any`] to specifically handle the type of this metric.
    Other,
}

/// A statically declared metric entry.
pub struct MetricEntry {
    metric: MetricWrapper,
    name: Cow<'static, str>,
    description: Option<Cow<'static, str>>,
    metadata: Metadata,
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

    /// Access the [`Metadata`] associated with this metrics entry.
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// Checks whether `metric` is the metric for this entry.
    ///
    /// This checks both the type id and the address. Note that it may have
    /// false positives if `metric` is a ZST since multiple ZSTs may share
    /// the same address.
    pub fn is(&self, metric: &dyn Metric) -> bool {
        if self.metric().type_id() != metric.type_id() {
            return false;
        }

        let a = self.metric() as *const _ as *const ();
        let b = metric as *const _ as *const ();
        a == b
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
