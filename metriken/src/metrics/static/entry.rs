use crate::*;

pub struct StaticEntry {
    metric: &'static dyn Metric,
    metadata: Metadata,
    name: &'static str,
    description: Option<&'static str>,
    formatter: &'static (dyn Fn(&dyn MetricEntry, Format) -> String + Sync),
}

impl StaticEntry {
    pub const fn new(
        metric: &'static dyn Metric,
        name: &'static str,
        description: &'static str,
        metadata: Metadata,
        formatter: &'static (dyn Fn(&dyn MetricEntry, Format) -> String + Sync),
    ) -> Self {
        let description = if description.is_empty() {
            None
        } else {
            Some(description)
        };

        Self {
            metric,
            metadata,
            name,
            description,
            formatter,
        }
    }
}

impl MetricEntry for StaticEntry {
    fn get_label(&self, label: &str) -> Option<&str> {
        self.metadata.map.get(label).copied()
    }
    fn format(&self, format: Format) -> std::string::String {
        (self.formatter)(self, format)
    }
    fn metadata(&self) -> HashMap<&str, &str> {
        let mut ret = HashMap::new();
        for (k, v) in self.metadata.map.entries {
            ret.insert(*k, *v);
        }
        ret
    }
    fn name(&self) -> &str {
        self.name
    }
    fn description(&self) -> Option<&str> {
        self.description
    }
}

impl Deref for StaticEntry {
    type Target = dyn Metric;

    fn deref(&self) -> &<Self as std::ops::Deref>::Target {
        self.metric
    }
}
