use crate::Metric;

/// A metric that is reported as a blank slate.
///
/// Used internally by `MetricBuilder` when creating a new `MetricEntry`.
pub(crate) struct NullMetric;

impl Metric for NullMetric {
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        None
    }
}
