use crate::*;

pub struct StaticMetric<T: 'static> {
    #[doc(hidden)]
    pub metric: &'static T,
}

impl<T: Metric> Metric for StaticMetric<T> {
    fn as_any(&self) -> &(dyn std::any::Any + 'static) {
        self.metric.as_any()
    }
}

impl<T: Metric> Deref for StaticMetric<T> {
    type Target = T;

    fn deref(&self) -> &<Self as std::ops::Deref>::Target {
        self.metric
    }
}
