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
//! the [`metrics()`] function. This will return an instance of the [`Metric`]
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

use crate::export::WrapMetric;

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
mod formatter;
mod gauge;
pub mod histogram;
mod lazy;
mod metrics;
mod null;

extern crate self as metriken;

pub mod dynmetrics;

pub use crate::counter::Counter;
pub use crate::dynmetrics::{DynBoxedMetric, DynPinnedMetric, MetricBuilder};
pub use crate::formatter::{default_formatter, Format};
pub use crate::gauge::Gauge;
pub use crate::histogram::{AtomicHistogram, RwLockHistogram};
pub use crate::lazy::Lazy;
pub use crate::metrics::{metrics, DynMetricsIter, Metrics, MetricsIter};

#[doc(inline)]
pub use metriken_core::{Metadata, MetadataIter};
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
    pub use metriken_core::declare_metric_v1;

    use crate::Metric;

    use std::ops::{Deref, DerefMut};

    pub struct WrapMetric<T>(T);

    impl<T> WrapMetric<T> {
        pub const fn new(value: T) -> Self {
            Self(value)
        }
    }

    impl<T> Deref for WrapMetric<T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<T> DerefMut for WrapMetric<T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    impl<T: Metric> metriken_core::Metric for WrapMetric<T> {
        fn is_enabled(&self) -> bool {
            <T as Metric>::is_enabled(self)
        }

        fn as_any(&self) -> Option<&dyn std::any::Any> {
            <T as Metric>::as_any(self)
        }

        fn value(&self) -> Option<metriken_core::Value> {
            let value = <T as Metric>::value(self)?;

            Some(match value {
                crate::Value::Counter(val) => metriken_core::Value::Counter(val),
                crate::Value::Gauge(val) => metriken_core::Value::Gauge(val),
                _ => metriken_core::Value::Other(self),
            })
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

    AtomicHistogram(&'a AtomicHistogram),
    RwLockHistogram(&'a RwLockHistogram),

    /// The value of the metric could not be represented using the other
    /// `Value` variants.
    ///
    /// Use [`Metric::as_any`] to specifically handle the type of this metric.
    Other,
}

/// A statically declared metric entry.
#[repr(transparent)]
pub struct MetricEntry(metriken_core::MetricEntry);

impl MetricEntry {
    /// Get a reference to the metric that this entry corresponds to.
    pub fn metric(&self) -> &CoreMetric {
        CoreMetric::from_core(self.0.metric())
    }

    /// Get the name of this metric.
    pub fn name(&self) -> &str {
        self.0.name()
    }

    /// Get the description of this metric.
    pub fn description(&self) -> Option<&str> {
        self.0.description()
    }

    /// Access the [`Metadata`] associated with this metrics entry.
    pub fn metadata(&self) -> &Metadata {
        self.0.metadata()
    }

    /// Format the metric into a string with the given format.
    pub fn formatted(&self, format: Format) -> String {
        self.0.formatted(format)
    }

    /// Checks whether `metric` is the metric for this entry.
    ///
    /// This checks both the type id and the address. Note that it may have
    /// false positives if `metric` is a ZST since multiple ZSTs may share
    /// the same address.
    pub fn is(&self, metric: &dyn Metric) -> bool {
        let a = self.metric() as *const _ as *const ();
        let b = metric as *const _ as *const ();
        a == b
    }

    #[doc(hidden)]
    pub fn from_core(core: &metriken_core::MetricEntry) -> &Self {
        // SAFETY: We are a #[repr(transparent)] wrapper around a MetricEntry
        //         so this is safe.
        unsafe { std::mem::transmute(core) }
    }

    #[doc(hidden)]
    pub fn as_core(&self) -> &metriken_core::MetricEntry {
        &self.0
    }
}

unsafe impl Send for MetricEntry {}
unsafe impl Sync for MetricEntry {}

impl std::ops::Deref for MetricEntry {
    type Target = CoreMetric;

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

impl<T: metriken_core::Metric> Metric for T {
    fn is_enabled(&self) -> bool {
        <T as metriken_core::Metric>::is_enabled(self)
    }

    fn as_any(&self) -> Option<&dyn Any> {
        <T as metriken_core::Metric>::as_any(self)
    }

    fn value(&self) -> Option<Value> {
        use metriken_core::Value as CoreValue;

        Some(match <T as metriken_core::Metric>::value(self)? {
            CoreValue::Counter(val) => Value::Counter(val),
            CoreValue::Gauge(val) => Value::Gauge(val),
            CoreValue::Other(val) => {
                if let Some(histogram) = val.downcast_ref::<AtomicHistogram>() {
                    Value::AtomicHistogram(histogram)
                } else if let Some(histogram) = val.downcast_ref::<RwLockHistogram>() {
                    Value::RwLockHistogram(histogram)
                } else {
                    Value::Other
                }
            }
            _ => Value::Other,
        })
    }
}

#[repr(transparent)]
pub struct CoreMetric(dyn metriken_core::Metric);

impl CoreMetric {
    fn from_core(core: &dyn metriken_core::Metric) -> &Self {
        // SAFETY: We are #[repr(transparent)] so this is safe.
        unsafe { std::mem::transmute(core) }
    }

    pub fn is_enabled(&self) -> bool {
        <Self as metriken_core::Metric>::is_enabled(self)
    }

    pub fn as_any(&self) -> Option<&dyn Any> {
        <Self as metriken_core::Metric>::as_any(self)
    }

    pub fn value(&self) -> Option<Value> {
        use metriken_core::Value as CoreValue;

        Some(match <Self as metriken_core::Metric>::value(self)? {
            CoreValue::Counter(val) => Value::Counter(val),
            CoreValue::Gauge(val) => Value::Gauge(val),
            CoreValue::Other(val) => {
                if let Some(histogram) = val.downcast_ref::<AtomicHistogram>() {
                    Value::AtomicHistogram(histogram)
                } else if let Some(histogram) = val.downcast_ref::<RwLockHistogram>() {
                    Value::RwLockHistogram(histogram)
                } else {
                    Value::Other
                }
            }
            _ => Value::Other,
        })
    }
}

impl metriken_core::Metric for CoreMetric {
    fn is_enabled(&self) -> bool {
        self.0.is_enabled()
    }

    fn as_any(&self) -> Option<&dyn Any> {
        self.0.as_any()
    }

    fn value(&self) -> Option<metriken_core::Value> {
        self.0.value()
    }
}
