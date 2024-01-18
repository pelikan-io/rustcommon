use crate::MetricEntry;

#[doc(inline)]
pub use metriken_core::Format;

/// The default formatter supports Prometheus-style exposition, and otherwise
/// simply prints the metric name.
pub fn default_formatter(metric: &MetricEntry, format: Format) -> String {
    metriken_core::default_formatter(metric.as_core(), format)
}
