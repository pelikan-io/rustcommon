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

pub use casey::lower;
pub use metriken_derive::metric;
pub use phf::phf_map;

mod counters;
mod gauges;
mod heatmap;
mod metrics;

pub use crate::heatmap::Heatmap;
pub use counters::{Counter, LazyCounter};
pub use gauges::{Gauge, LazyGauge};
pub(crate) use metrics::DynamicRegistry;
pub use metrics::{
    DynamicEntry, DynamicMetric, DynamicMetricBuilder, Metric, MetricEntry, MetricIterator,
    Metrics, StaticEntry, StaticMetric,
};

pub(crate) static DYNAMIC_REGISTRY: DynamicRegistry = DynamicRegistry::new();

#[linkme::distributed_slice]
pub static STATIC_ENTRIES: [StaticEntry] = [..];

pub enum Format {
    Plain,
    Prometheus,
}

pub fn metrics() -> Metrics {
    Metrics {
        dynamic: DYNAMIC_REGISTRY.read(),
    }
}

pub fn deregister_all() {
    Metrics::deregister_all()
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
                .filter(|(k, _v)| {
                    **k != "name" && **k != "description"
                })
                .map(|(k, v)| {
                    format!("{k}=\"{v}\"")
                })
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
        metriken::Metadata::new(metriken::phf_map!($($tts)*))
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
        Self { map: phf_map!() }
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

#[cfg(test)]
mod tests {
    use crate::*;
    use parking_lot::const_mutex;
    use parking_lot::Mutex;
    use parking_lot::MutexGuard;

    static MUTEX: Mutex<()> = const_mutex(());

    struct Guard {
        _lock: MutexGuard<'static, ()>,
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            Metrics::deregister_all()
        }
    }

    #[test]
    fn dynamic_counters() {
        let _guard = MUTEX.lock();

        // the registry is empty
        assert_eq!(DYNAMIC_REGISTRY.len(), 0);

        // define a new counter
        let a = DynamicMetric::builder(Counter::new(), "counter-a").build();

        // show that the counter is added and functions as expected

        assert_eq!(a.value(), 0);
        assert_eq!(DYNAMIC_REGISTRY.len(), 1);

        assert_eq!(a.increment(), 0);
        assert_eq!(a.value(), 1);
        assert_eq!(a.add(2), 1);
        assert_eq!(a.value(), 3);

        // add another counter
        let b = DynamicMetric::builder(Counter::new(), "counter-b").build();

        // show that the new gauge is added and functions independently

        assert_eq!(DYNAMIC_REGISTRY.len(), 2);
        assert_eq!(a.value(), 3);

        assert_eq!(b.value(), 0);
        assert_eq!(b.add(10), 0);
        assert_eq!(b.value(), 10);

        assert_eq!(a.value(), 3);

        // drop one of the counters and see that the registry length reflects
        // successful deregistration

        drop(a);

        assert_eq!(DYNAMIC_REGISTRY.len(), 1);
    }

    #[test]
    fn dynamic_gauges() {
        let _guard = MUTEX.lock();

        // the registry is empty
        assert_eq!(DYNAMIC_REGISTRY.len(), 0);

        // define a new gauge
        let a = DynamicMetric::builder(Gauge::new(), "gauge-a").build();

        // show that the counter is added and functions as expected

        assert_eq!(a.value(), 0);
        assert_eq!(DYNAMIC_REGISTRY.len(), 1);

        assert_eq!(a.increment(), 0);
        assert_eq!(a.value(), 1);
        assert_eq!(a.add(2), 1);
        assert_eq!(a.value(), 3);

        assert_eq!(a.decrement(), 3);
        assert_eq!(a.value(), 2);
        assert_eq!(a.sub(3), 2);
        assert_eq!(a.value(), -1);

        // add another gauge
        let b = DynamicMetric::builder(Counter::new(), "gauge-b").build();

        // show that the new gauge is added and functions independently

        assert_eq!(DYNAMIC_REGISTRY.len(), 2);
        assert_eq!(a.value(), -1);

        assert_eq!(b.value(), 0);
        assert_eq!(b.add(10), 0);
        assert_eq!(b.value(), 10);

        assert_eq!(a.value(), -1);

        // drop one of the gauges and see that the registry length reflects
        // successful deregistration

        drop(a);

        assert_eq!(DYNAMIC_REGISTRY.len(), 1);
    }

    #[test]
    fn dynamic_lazy_counter() {
        let _guard = MUTEX.lock();

        // the registry is empty
        assert_eq!(DYNAMIC_REGISTRY.len(), 0);

        // add a lazy counter to the registry
        let a = DynamicMetric::builder(LazyCounter::default(), "counter-a").build();

        assert_eq!(DYNAMIC_REGISTRY.len(), 1);

        // since the counter has only been defined and not used, it remains
        // uninitialized

        for metric in &metrics() {
            if let Some(counter) = metric.as_any().downcast_ref::<LazyCounter>() {
                assert!(counter.value().is_none());
            } else {
                panic!("unexpected metric type");
            }
        }

        // after using the counter, the metric is initialized

        a.increment();

        for metric in &metrics() {
            if let Some(counter) = metric.as_any().downcast_ref::<LazyCounter>() {
                assert!(counter.value().is_some());
            } else {
                panic!("unexpected metric type");
            }
        }
    }

    #[test]
    fn format() {
        let _guard = MUTEX.lock();
        
        let _a = DynamicMetric::builder(Counter::new(), "counter").metadata("key", "value").build();
        let metrics = metrics();
        let entry = metrics.iter().next().unwrap();

        assert_eq!(entry.format(Format::Plain), Some("counter".to_string()));
        assert_eq!(entry.format(Format::Prometheus), Some("counter{key=\"value\"}".to_string()));
    }
}
