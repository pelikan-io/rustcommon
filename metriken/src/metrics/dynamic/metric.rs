use crate::*;

pub struct DynamicMetric<T: Metric> {
    metric: Arc<T>,
}

impl<T: Metric> Deref for DynamicMetric<T> {
    type Target = T;

    fn deref(&self) -> &<Self as std::ops::Deref>::Target {
        &self.metric
    }
}

pub struct DynamicMetricBuilder<T> {
    metric: T,
    metadata: HashMap<String, String>,
    name: String,
    description: Option<String>,
    formatter: &'static (dyn Fn(&dyn MetricEntry, Format) -> String + Sync),
}

impl<M: Metric> DynamicMetricBuilder<M> {
    pub fn build(self) -> DynamicMetric<M> {
        let metric = Arc::new(self.metric);

        let entry = DynamicEntry {
            metric: metric.clone(),
            metadata: self.metadata,
            name: self.name,
            description: self.description,
            formatter: self.formatter,
        };

        DYNAMIC_REGISTRY.register(entry);

        DynamicMetric { metric }
    }

    pub fn description<T: ToString>(self, description: T) -> Self {
        self.metadata("description", description)
    }

    pub fn metadata<K: ToString, V: ToString>(mut self, key: K, value: V) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }

    pub fn formatter(
        mut self,
        formatter: &'static (dyn Fn(&dyn MetricEntry, Format) -> String + Sync),
    ) -> Self {
        self.formatter = formatter;
        self
    }
}

impl<M: Metric> DynamicMetric<M> {
    pub fn builder<T: ToString>(metric: M, name: T) -> DynamicMetricBuilder<M> {
        let metadata: HashMap<String, String> = HashMap::new();

        DynamicMetricBuilder {
            metric,
            metadata,
            name: name.to_string(),
            description: None,
            formatter: &default_formatter,
        }
    }
}

impl<T: Metric> Metric for DynamicMetric<T> {
    fn as_any(&self) -> &(dyn std::any::Any + 'static) {
        (*self.metric).as_any()
    }
}

impl<T: Metric> Drop for DynamicMetric<T> {
    fn drop(&mut self) {
        // remove this metric from the registry
        DYNAMIC_REGISTRY.deregister(self.metric.clone());
    }
}
