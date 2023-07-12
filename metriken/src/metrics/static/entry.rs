use crate::*;

pub struct StaticEntry {
    metric: &'static dyn Metric,
    metadata: Metadata,
    formatter: &'static (dyn Fn(&dyn MetricEntry, Format) -> Option<String> + Sync),
}

impl StaticEntry {
    pub const fn new(
        metric: &'static dyn Metric,
        metadata: Metadata,
        formatter: &'static (dyn Fn(&dyn MetricEntry, Format) -> Option<String> + Sync),
    ) -> Self {
        Self {
            metric,
            metadata,
            formatter,
        }
    }
}

impl MetricEntry for StaticEntry {
    fn get_label(&self, label: &str) -> Option<&str> {
        self.metadata.map.get(label).copied()
    }
    fn format(&self, format: Format) -> std::option::Option<std::string::String> {
        (self.formatter)(self, format)
    }
    fn metadata(&self) -> HashMap<&str, &str> {
        let mut ret = HashMap::new();
        for (k, v) in self.metadata.map.entries {
            ret.insert(*k, *v);
        }
        ret
    }
}

impl Deref for StaticEntry {
    type Target = dyn Metric;

    fn deref(&self) -> &<Self as std::ops::Deref>::Target {
        self.metric
    }
}
