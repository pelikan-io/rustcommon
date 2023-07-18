use crate::*;

pub(crate) struct DynamicRegistry {
    metrics: RwLock<Vec<DynamicEntry>>,
}

impl DynamicRegistry {
    pub(crate) const fn new() -> Self {
        Self {
            metrics: RwLock::new(Vec::new()),
        }
    }

    pub(crate) fn register(&self, entry: DynamicEntry) {
        let mut metrics = self.metrics.write();
        metrics.push(entry);
    }

    pub(crate) fn deregister(&self, metric: Arc<dyn Metric>) {
        let mut metrics = self.metrics.write();

        metrics.retain(|entry| {
            // compares only the data addresses of the wide pointers
            Arc::as_ptr(&entry.metric) as *const () != Arc::as_ptr(&metric) as *const ()
        });
    }

    pub(crate) fn read(&self) -> RwLockReadGuard<Vec<DynamicEntry>> {
        self.metrics.read()
    }
}
