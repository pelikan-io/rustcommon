use crate::{Metric, MetricEntry};

used_in_docs!(MetricEntry);

/// A metric that always reports itself as disabled.
///
/// This is used as a default metric pointer within [`MetricEntry`] for cases
/// where there is no valid metric.
pub(crate) struct NullMetric;

impl Metric for NullMetric {
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        None
    }

    fn value(&self) -> Option<crate::Value> {
        None
    }
}
