//! Metriken is a metrics library with a focus on providing fast and lightweight
//! metrics for Rust programs and libraries.
//!
//! Unlike other metrics libraries, Metriken allows for static definitions of
//! metrics. This creates an easy-to-use metrics infrastructure with extremely
//! low overheads. Metriken also allows for defining metrics dynamically at
//! runtime.
//!
//! Metrics can have associated metadata in the form of key-value pairs as well
//! as a custom formatting function. This allows for richer annotations for
//! observability systems that can handle key-value labels. It also allows
//! special formatting for more traditional observability systems where some
//! of the metadata needs to be encoded into the metric name for exposition.
//!
//! Metriken provides three kinds of metrics storage:
//! * counters - for monotonically non-decreasing values
//! * gauges - for values which may increase or decrease
//! * heatmaps - moving histograms which track a quantized full-distribution
//!   of value-count pairs for a given window in time

use core::any::Any;
use core::ops::Deref;
use core::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use parking_lot::RwLock;
use parking_lot::RwLockReadGuard;
use phf::Map;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;

pub use metriken_derive::metric;

mod counters;
mod gauges;
mod heatmap;
mod lazy;
mod metrics;

pub use crate::heatmap::Heatmap;
pub use counters::{Counter, LazyCounter};
pub use gauges::{Gauge, LazyGauge};

pub use metrics::{
    DynamicEntry, DynamicMetric, DynamicMetricBuilder, MetricEntry, MetricIterator,
    Metrics, StaticEntry, StaticMetric,
};

pub(crate) use lazy::Lazy;
pub(crate) use metrics::DynamicRegistry;
pub(crate) static DYNAMIC_REGISTRY: DynamicRegistry = DynamicRegistry::new();

#[doc(hidden)]
pub mod __private {
    pub extern crate linkme;
    pub use phf::phf_map;

    #[linkme::distributed_slice]
    pub static STATIC_REGISTRY: [crate::StaticEntry] = [..];
}

pub trait Metric: Send + Sync + 'static {
    fn as_any(&self) -> &dyn Any;
}

impl<T: Send + Sync> Metric for &'static T {
    fn as_any(&self) -> &(dyn std::any::Any + 'static) {
        self
    }
}

pub enum Format {
    Plain,
    Prometheus,
}

pub fn metrics() -> Metrics {
    Metrics {
        dynamic: DYNAMIC_REGISTRY.read(),
    }
}

pub fn default_formatter(metric: &dyn MetricEntry, format: Format) -> Option<String> {
    match format {
        Format::Plain => {
            // the default format is to just return the metric name
            metric.name().map(|name| name.to_string())
        }
        Format::Prometheus => {
            // prometheus format is the name followed by each metadata entry
            // as a label (filtering: `name` and `description` entries)
            let metadata = metric
                .metadata()
                .iter()
                .filter(|(k, _v)| **k != "name" && **k != "description")
                .map(|(k, v)| format!("{k}=\"{v}\""))
                .collect::<Vec<_>>()
                .join(",");
            metric.name().map(|name| format!("{name}{{{metadata}}}"))
        }
    }
}

#[macro_export]
#[rustfmt::skip]
macro_rules! metadata {
    ($($tts:tt)*) => {
        $crate::Metadata::new($crate::__private::phf_map!($($tts)*))
    };
}

pub struct Metadata {
    map: Map<&'static str, &'static str>,
}

impl Metadata {
    pub const fn new(map: Map<&'static str, &'static str>) -> Self {
        Self { map }
    }

    pub const fn empty() -> Self {
        Self { map: __private::phf_map!() }
    }
}

impl Metadata {
    pub fn name(&self) -> Option<&str> {
        self.get_label("name")
    }

    pub fn help(&self) -> Option<&str> {
        self.get_label("help")
    }

    pub fn get_label(&self, label: &'static str) -> Option<&str> {
        self.map.get(label).copied()
    }
}


