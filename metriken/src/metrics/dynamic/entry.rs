use crate::*;

pub struct DynamicEntry {
    pub(crate) metric: Arc<dyn Metric>,
    pub(crate) metadata: HashMap<String, String>,
    pub(crate) formatter: &'static (dyn Fn(&dyn MetricEntry, Format) -> Option<String> + Sync),
}

impl std::cmp::PartialEq for DynamicEntry {
    // NOTE: we want to compare only the data addresses of the wide pointers.
    #[allow(clippy::ptr_eq)]
    fn eq(&self, other: &DynamicEntry) -> bool {
        Arc::as_ptr(&self.metric) as *const () == Arc::as_ptr(&other.metric) as *const ()
    }
}

impl std::cmp::Eq for DynamicEntry {}

impl std::cmp::PartialOrd for DynamicEntry {
    fn partial_cmp(&self, other: &DynamicEntry) -> std::option::Option<std::cmp::Ordering> {
        (Arc::as_ptr(&self.metric) as *const ())
            .partial_cmp(&(Arc::as_ptr(&other.metric) as *const ()))
    }
}

impl std::cmp::Ord for DynamicEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (Arc::as_ptr(&self.metric) as *const ()).cmp(&(Arc::as_ptr(&other.metric) as *const ()))
    }
}

impl MetricEntry for DynamicEntry {
    fn get_label(&self, label: &str) -> Option<&str> {
        self.metadata.get(label).map(|v| v.as_str())
    }

    fn metadata(&self) -> HashMap<&str, &str> {
        let mut ret = HashMap::new();
        for (key, value) in &self.metadata {
            ret.insert(key.as_str(), value.as_str());
        }
        ret
    }

    fn format(&self, format: Format) -> std::option::Option<std::string::String> {
        (self.formatter)(self, format)
    }
}

impl Deref for DynamicEntry {
    type Target = dyn Metric;

    fn deref(&self) -> &<Self as std::ops::Deref>::Target {
        self.metric.borrow()
    }
}
